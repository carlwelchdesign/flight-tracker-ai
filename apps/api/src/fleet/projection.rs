use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};

use thiserror::Error;
use tokio::sync::{RwLock, broadcast};

use crate::{
    domain::{
        AircraftPosition, CanonicalEvent, Flight, FlightId, OperatorId, ProviderEnvelope,
        ProviderEnvelopeId, SourceAttribution,
    },
    health::WorkerProbe,
    ingestion::NormalizedEventBatch,
};

use super::{FleetEvent, FlightPage, FlightView, PageMetadata, TimelinePage};

#[derive(Debug, Clone)]
struct ProjectedFlight {
    flight: Flight,
    latest_position: Option<AircraftPosition>,
}

#[derive(Default)]
struct ProjectionState {
    flights: HashMap<FlightId, ProjectedFlight>,
    timelines: HashMap<FlightId, Vec<FleetEvent>>,
    retained_events: VecDeque<Arc<FleetEvent>>,
    seen_envelopes: HashSet<ProviderEnvelopeId>,
    next_event_id: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ApplyReport {
    pub accepted_events: usize,
    pub ignored_events: usize,
    pub duplicate_batch: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ApplyError {
    #[error("normalized batch cannot be empty")]
    EmptyBatch,
    #[error("event {index} does not match its provider envelope: {reason}")]
    InvalidEvent { index: usize, reason: &'static str },
}

#[derive(Clone)]
pub struct FleetStore {
    state: Arc<RwLock<ProjectionState>>,
    sender: broadcast::Sender<Arc<FleetEvent>>,
    retention: usize,
}

impl FleetStore {
    pub fn new(retention: usize) -> Self {
        assert!(retention > 0, "fleet event retention must be non-zero");
        let (sender, _) = broadcast::channel(retention);
        Self {
            state: Arc::new(RwLock::new(ProjectionState::default())),
            sender,
            retention,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<FleetEvent>> {
        self.sender.subscribe()
    }

    pub async fn apply(&self, batch: &NormalizedEventBatch) -> Result<ApplyReport, ApplyError> {
        validate_batch(batch)?;
        let mut state = self.state.write().await;
        if state.seen_envelopes.contains(&batch.envelope.id) {
            return Ok(ApplyReport {
                duplicate_batch: true,
                ignored_events: batch.events.len(),
                ..ApplyReport::default()
            });
        }

        let mut report = ApplyReport::default();
        let mut published = Vec::new();
        for canonical in &batch.events {
            let accepted = match canonical {
                CanonicalEvent::Flight(flight) => apply_flight(&mut state, flight),
                CanonicalEvent::AircraftPosition(position) => apply_position(&mut state, position),
                _ => true,
            };
            if !accepted {
                report.ignored_events += 1;
                continue;
            }

            state.next_event_id = state.next_event_id.saturating_add(1);
            let event = Arc::new(to_fleet_event(
                state.next_event_id,
                batch.envelope.id,
                canonical.clone(),
            ));
            if let Some(flight_id) = event.flight_id {
                state
                    .timelines
                    .entry(flight_id)
                    .or_default()
                    .push((*event).clone());
            }
            state.retained_events.push_back(event.clone());
            while state.retained_events.len() > self.retention {
                state.retained_events.pop_front();
            }
            published.push(event);
            report.accepted_events += 1;
        }
        state.seen_envelopes.insert(batch.envelope.id);
        drop(state);

        for event in published {
            let _ = self.sender.send(event);
        }
        Ok(report)
    }

    pub async fn list(&self, page: usize, page_size: usize) -> FlightPage {
        let state = self.state.read().await;
        let mut flights: Vec<_> = state
            .flights
            .values()
            .map(|projected| FlightView {
                flight: projected.flight.clone(),
                latest_position: projected.latest_position.clone(),
            })
            .collect();
        flights.sort_by(|left, right| {
            left.flight
                .callsign
                .cmp(&right.flight.callsign)
                .then_with(|| left.flight.id.as_uuid().cmp(&right.flight.id.as_uuid()))
        });
        let pagination = page_metadata(page, page_size, flights.len());
        FlightPage {
            data: paginate(&flights, page, page_size),
            pagination,
        }
    }

    pub async fn detail(&self, flight_id: FlightId) -> Option<FlightView> {
        self.state
            .read()
            .await
            .flights
            .get(&flight_id)
            .map(|projected| FlightView {
                flight: projected.flight.clone(),
                latest_position: projected.latest_position.clone(),
            })
    }

    pub async fn timeline(
        &self,
        flight_id: FlightId,
        page: usize,
        page_size: usize,
    ) -> Option<TimelinePage> {
        let state = self.state.read().await;
        if !state.flights.contains_key(&flight_id) {
            return None;
        }
        let mut events = state.timelines.get(&flight_id).cloned().unwrap_or_default();
        events.sort_by_key(|event| (event.event_time, event.id));
        let pagination = page_metadata(page, page_size, events.len());
        Some(TimelinePage {
            data: paginate(&events, page, page_size),
            pagination,
        })
    }

    pub async fn events_after(&self, event_id: u64) -> Vec<Arc<FleetEvent>> {
        self.state
            .read()
            .await
            .retained_events
            .iter()
            .filter(|event| event.id > event_id)
            .cloned()
            .collect()
    }

    pub async fn clear_projection(&self) {
        let mut state = self.state.write().await;
        let next_event_id = state.next_event_id;
        *state = ProjectionState {
            next_event_id,
            ..ProjectionState::default()
        };
    }
}

pub fn spawn_projection_worker(
    store: FleetStore,
    mut receiver: broadcast::Receiver<Arc<NormalizedEventBatch>>,
    probe: WorkerProbe,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut heartbeat = tokio::time::interval(Duration::from_secs(1));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = heartbeat.tick() => probe.heartbeat(),
                received = receiver.recv() => match received {
                    Ok(batch) => {
                        probe.heartbeat();
                        match store.apply(&batch).await {
                            Ok(report) => tracing::debug!(
                                correlation_id = %batch.envelope.id.as_uuid(),
                                accepted_events = report.accepted_events,
                                ignored_events = report.ignored_events,
                                duplicate_batch = report.duplicate_batch,
                                "fleet projection applied ingestion batch"
                            ),
                            Err(error) => tracing::warn!(
                                correlation_id = %batch.envelope.id.as_uuid(),
                                error = %error,
                                "fleet projection rejected batch"
                            ),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::error!(skipped, "fleet projection lagged behind ingestion");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    })
}

fn validate_batch(batch: &NormalizedEventBatch) -> Result<(), ApplyError> {
    if batch.events.is_empty() {
        return Err(ApplyError::EmptyBatch);
    }
    for (index, event) in batch.events.iter().enumerate() {
        let (operator_id, source) = event_identity(event);
        if operator_id != batch.envelope.operator_id {
            return Err(ApplyError::InvalidEvent {
                index,
                reason: "operator differs",
            });
        }
        if let Some(source) = source
            && !source_matches_envelope(source, &batch.envelope)
        {
            return Err(ApplyError::InvalidEvent {
                index,
                reason: "source attribution differs",
            });
        }
    }
    Ok(())
}

fn event_identity(event: &CanonicalEvent) -> (OperatorId, Option<&SourceAttribution>) {
    match event {
        CanonicalEvent::Flight(value) => (value.operator_id, Some(&value.source)),
        CanonicalEvent::AircraftPosition(value) => (value.operator_id, Some(&value.source)),
        CanonicalEvent::PlannedRoute(value) => (value.operator_id, Some(&value.source)),
        CanonicalEvent::AirportObservation(value) => (value.operator_id, Some(&value.source)),
        CanonicalEvent::WeatherHazard(value) => (value.operator_id, Some(&value.source)),
        CanonicalEvent::Alert(value) => (value.operator_id, None),
        CanonicalEvent::AlertAction(value) => (value.operator_id, None),
        CanonicalEvent::SourceHealth(value) => (value.operator_id, None),
    }
}

fn source_matches_envelope(source: &SourceAttribution, envelope: &ProviderEnvelope) -> bool {
    source.envelope_id == envelope.id
        && source.provider == envelope.provider
        && source.feed == envelope.feed
        && source.provider_record_id == envelope.provider_record_id
}

fn apply_flight(state: &mut ProjectionState, flight: &Flight) -> bool {
    match state.flights.get_mut(&flight.id) {
        Some(current) if flight.times.event_time <= current.flight.times.event_time => false,
        Some(current) => {
            current.flight = flight.clone();
            true
        }
        None => {
            state.flights.insert(
                flight.id,
                ProjectedFlight {
                    flight: flight.clone(),
                    latest_position: None,
                },
            );
            true
        }
    }
}

fn apply_position(state: &mut ProjectionState, position: &AircraftPosition) -> bool {
    let Some(current) = state.flights.get_mut(&position.flight_id) else {
        return false;
    };
    if current.flight.operator_id != position.operator_id
        || current
            .latest_position
            .as_ref()
            .is_some_and(|existing| position.times.event_time <= existing.times.event_time)
    {
        return false;
    }
    current.latest_position = Some(position.clone());
    true
}

fn to_fleet_event(id: u64, envelope_id: ProviderEnvelopeId, event: CanonicalEvent) -> FleetEvent {
    let (flight_id, event_time, source) = match &event {
        CanonicalEvent::Flight(value) => (
            Some(value.id),
            value.times.event_time,
            Some(value.source.clone()),
        ),
        CanonicalEvent::AircraftPosition(value) => (
            Some(value.flight_id),
            value.times.event_time,
            Some(value.source.clone()),
        ),
        CanonicalEvent::PlannedRoute(value) => (
            Some(value.flight_id),
            value.times.event_time,
            Some(value.source.clone()),
        ),
        CanonicalEvent::AirportObservation(value) => {
            (None, value.times.event_time, Some(value.source.clone()))
        }
        CanonicalEvent::WeatherHazard(value) => {
            (None, value.times.event_time, Some(value.source.clone()))
        }
        CanonicalEvent::Alert(value) => (value.flight_id, value.times.event_time, None),
        CanonicalEvent::AlertAction(value) => (None, value.occurred_at, None),
        CanonicalEvent::SourceHealth(value) => (None, value.observed_at, None),
    };
    FleetEvent {
        id,
        flight_id,
        envelope_id,
        event_time,
        source,
        event,
    }
}

fn page_metadata(page: usize, page_size: usize, total_items: usize) -> PageMetadata {
    PageMetadata {
        page,
        page_size,
        total_items,
        total_pages: total_items.div_ceil(page_size),
    }
}

fn paginate<T: Clone>(values: &[T], page: usize, page_size: usize) -> Vec<T> {
    let start = page.saturating_sub(1).saturating_mul(page_size);
    values.iter().skip(start).take(page_size).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::CanonicalEvent, replay::ReplayScenario};

    fn fixture() -> ReplayScenario {
        ReplayScenario::from_json(include_str!(
            "../../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap()
    }

    #[tokio::test]
    async fn duplicate_and_out_of_order_events_do_not_corrupt_current_state() {
        let scenario = fixture();
        let store = FleetStore::new(32);
        let flight_batch = scenario.batch_for(&scenario.events[1]).unwrap();
        store.apply(&flight_batch).await.unwrap();
        assert!(store.apply(&flight_batch).await.unwrap().duplicate_batch);

        let later = scenario.batch_for(&scenario.events[8]).unwrap();
        store.apply(&later).await.unwrap();
        let earlier = scenario.batch_for(&scenario.events[5]).unwrap();
        let report = store.apply(&earlier).await.unwrap();
        assert_eq!(report.ignored_events, 1);

        let flight_id = scenario.flights[0].id;
        let detail = store.detail(flight_id).await.unwrap();
        assert_eq!(
            detail.latest_position.unwrap().times.event_time,
            scenario.start_time + chrono::Duration::seconds(60)
        );
        assert_eq!(
            store.timeline(flight_id, 1, 10).await.unwrap().data.len(),
            2
        );
    }

    #[tokio::test]
    async fn invalid_source_attribution_rejects_the_whole_batch() {
        let scenario = fixture();
        let store = FleetStore::new(32);
        let mut batch = scenario.batch_for(&scenario.events[1]).unwrap();
        if let CanonicalEvent::Flight(flight) = &mut batch.events[0] {
            flight.source.provider = "unexpected".into();
        }

        assert!(matches!(
            store.apply(&batch).await,
            Err(ApplyError::InvalidEvent { .. })
        ));
        assert_eq!(store.list(1, 10).await.pagination.total_items, 0);
    }

    #[tokio::test]
    async fn list_and_timeline_are_stably_paginated() {
        let scenario = fixture();
        let store = FleetStore::new(32);
        for event in &scenario.events {
            store
                .apply(&scenario.batch_for(event).unwrap())
                .await
                .unwrap();
        }

        let first = store.list(1, 2).await;
        let second = store.list(2, 2).await;
        assert_eq!(first.pagination.total_items, 3);
        assert_eq!(first.pagination.total_pages, 2);
        assert_eq!(first.data.len(), 2);
        assert_eq!(second.data.len(), 1);
        assert!(first.data[0].flight.callsign <= first.data[1].flight.callsign);

        let timeline = store.timeline(scenario.flights[0].id, 1, 10).await.unwrap();
        assert!(timeline.data.windows(2).all(|events| {
            (events[0].event_time, events[0].id) < (events[1].event_time, events[1].id)
        }));
        assert!(timeline.data.iter().all(|event| event.source.is_some()));
    }

    #[tokio::test]
    async fn reset_clears_state_and_deduplication_but_keeps_event_ids_monotonic() {
        let scenario = fixture();
        let store = FleetStore::new(32);
        let batch = scenario.batch_for(&scenario.events[1]).unwrap();
        store.apply(&batch).await.unwrap();
        assert_eq!(store.events_after(0).await[0].id, 1);

        store.clear_projection().await;
        assert_eq!(store.list(1, 10).await.pagination.total_items, 0);
        assert!(store.events_after(0).await.is_empty());

        assert_eq!(store.apply(&batch).await.unwrap().accepted_events, 1);
        assert_eq!(store.events_after(0).await[0].id, 2);
    }
}
