use std::{collections::HashSet, fs, path::Path};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        AircraftPosition, AircraftPositionId, Altitude, AltitudeBand, CanonicalEvent, EventTimes,
        Flight, FlightId, FlightStatus, GeoPoint, GeoPolygon, HazardSeverity, HeadingDegrees,
        OperatorId, ProviderEnvelope, ProviderEnvelopeId, SchemaVersion, SourceAttribution,
        SourceQuality, Speed, WeatherHazard, WeatherHazardId, WeatherHazardStatus,
    },
    ingestion::NormalizedEventBatch,
};

const SUPPORTED_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightRole {
    Normal,
    Delayed,
    HazardAdjacent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioFlight {
    pub id: FlightId,
    pub role: FlightRole,
    pub callsign: String,
    pub aircraft_registration: String,
    pub origin_airport_code: String,
    pub destination_airport_code: String,
    pub scheduled_departure_at: DateTime<Utc>,
    pub scheduled_arrival_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioEvent {
    pub sequence: u32,
    pub offset_ms: u64,
    pub provider_record_id: String,
    #[serde(flatten)]
    pub payload: ScenarioPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum ScenarioPayload {
    FlightSnapshot {
        flight_id: FlightId,
        status: FlightStatus,
    },
    Position {
        flight_id: FlightId,
        point: GeoPoint,
        altitude: Option<Altitude>,
        heading_true_degrees: Option<HeadingDegrees>,
        ground_speed: Option<Speed>,
        quality: SourceQuality,
    },
    WeatherHazard {
        hazard_id: WeatherHazardId,
        hazard_type: String,
        severity: HazardSeverity,
        valid_from_offset_ms: u64,
        valid_to_offset_ms: u64,
        altitude_band: Option<AltitudeBand>,
        footprint: GeoPolygon,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayScenario {
    pub schema_version: u16,
    pub id: String,
    pub namespace_id: Uuid,
    pub operator_id: OperatorId,
    pub start_time: DateTime<Utc>,
    pub flights: Vec<ScenarioFlight>,
    pub events: Vec<ScenarioEvent>,
}

#[derive(Debug, Error)]
pub enum ScenarioError {
    #[error("failed to read replay scenario: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid replay scenario JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported replay schema version {0}")]
    UnsupportedVersion(u16),
    #[error("invalid replay scenario: {0}")]
    Validation(String),
}

impl ReplayScenario {
    pub fn from_json(json: &str) -> Result<Self, ScenarioError> {
        let scenario: Self = serde_json::from_str(json)?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ScenarioError> {
        Self::from_json(&fs::read_to_string(path)?)
    }

    pub fn validate(&self) -> Result<(), ScenarioError> {
        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(ScenarioError::UnsupportedVersion(self.schema_version));
        }
        if self.id.trim().is_empty() || self.flights.is_empty() || self.events.is_empty() {
            return Err(ScenarioError::Validation(
                "id, flights, and events must not be empty".into(),
            ));
        }

        let flight_ids: HashSet<_> = self.flights.iter().map(|flight| flight.id).collect();
        if flight_ids.len() != self.flights.len() {
            return Err(ScenarioError::Validation(
                "flight IDs must be unique".into(),
            ));
        }

        let mut sequences = HashSet::new();
        let mut previous = None;
        for event in &self.events {
            if !sequences.insert(event.sequence) {
                return Err(ScenarioError::Validation(format!(
                    "event sequence {} is duplicated",
                    event.sequence
                )));
            }
            let ordering = (event.offset_ms, event.sequence);
            if previous.is_some_and(|value| value >= ordering) {
                return Err(ScenarioError::Validation(
                    "events must be strictly ordered by offset_ms and sequence".into(),
                ));
            }
            previous = Some(ordering);

            match &event.payload {
                ScenarioPayload::FlightSnapshot { flight_id, .. }
                | ScenarioPayload::Position { flight_id, .. } => {
                    if !flight_ids.contains(flight_id) {
                        return Err(ScenarioError::Validation(format!(
                            "event {} references an unknown flight",
                            event.sequence
                        )));
                    }
                }
                ScenarioPayload::WeatherHazard {
                    valid_from_offset_ms,
                    valid_to_offset_ms,
                    footprint,
                    ..
                } => {
                    if valid_to_offset_ms <= valid_from_offset_ms {
                        return Err(ScenarioError::Validation(format!(
                            "hazard event {} has an invalid validity window",
                            event.sequence
                        )));
                    }
                    if footprint.exterior.len() < 4
                        || footprint.exterior.first() != footprint.exterior.last()
                    {
                        return Err(ScenarioError::Validation(format!(
                            "hazard event {} must have a closed polygon",
                            event.sequence
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn batch_for(&self, event: &ScenarioEvent) -> Result<NormalizedEventBatch, ScenarioError> {
        let timestamp = offset_time(self.start_time, event.offset_ms)?;
        let raw_payload = serde_json::to_value(&event.payload)?;
        let raw_bytes = serde_json::to_vec(&raw_payload)?;
        let envelope_id = ProviderEnvelopeId::from_uuid(Uuid::new_v5(
            &self.namespace_id,
            format!("envelope:{}", event.sequence).as_bytes(),
        ));
        let source = SourceAttribution {
            envelope_id,
            provider: "simulation".into(),
            feed: self.id.clone(),
            provider_record_id: Some(event.provider_record_id.clone()),
        };
        let times = EventTimes::new(timestamp, timestamp, timestamp)
            .expect("equal replay timestamps are valid");
        let envelope = ProviderEnvelope {
            id: envelope_id,
            operator_id: self.operator_id,
            schema_version: SchemaVersion::V1,
            provider: source.provider.clone(),
            feed: source.feed.clone(),
            provider_record_id: source.provider_record_id.clone(),
            event_time: Some(timestamp),
            received_at: timestamp,
            processed_at: Some(timestamp),
            raw_payload_sha256: format!("{:x}", Sha256::digest(raw_bytes)),
            raw_payload,
        };

        let canonical = match &event.payload {
            ScenarioPayload::FlightSnapshot { flight_id, status } => {
                let flight = self
                    .flights
                    .iter()
                    .find(|candidate| candidate.id == *flight_id)
                    .expect("validated flight reference");
                CanonicalEvent::Flight(Flight {
                    id: *flight_id,
                    operator_id: self.operator_id,
                    schema_version: SchemaVersion::V1,
                    source,
                    times,
                    callsign: Some(flight.callsign.clone()),
                    aircraft_registration: Some(flight.aircraft_registration.clone()),
                    origin_airport_code: Some(flight.origin_airport_code.clone()),
                    destination_airport_code: Some(flight.destination_airport_code.clone()),
                    scheduled_departure_at: Some(flight.scheduled_departure_at),
                    scheduled_arrival_at: Some(flight.scheduled_arrival_at),
                    status: *status,
                })
            }
            ScenarioPayload::Position {
                flight_id,
                point,
                altitude,
                heading_true_degrees,
                ground_speed,
                quality,
            } => CanonicalEvent::AircraftPosition(AircraftPosition {
                id: AircraftPositionId::from_uuid(Uuid::new_v5(
                    &self.namespace_id,
                    format!("position:{}", event.sequence).as_bytes(),
                )),
                operator_id: self.operator_id,
                flight_id: *flight_id,
                schema_version: SchemaVersion::V1,
                source,
                times,
                point: *point,
                altitude: *altitude,
                heading_true_degrees: *heading_true_degrees,
                ground_speed: *ground_speed,
                quality: *quality,
            }),
            ScenarioPayload::WeatherHazard {
                hazard_id,
                hazard_type,
                severity,
                valid_from_offset_ms,
                valid_to_offset_ms,
                altitude_band,
                footprint,
            } => CanonicalEvent::WeatherHazard(WeatherHazard {
                id: *hazard_id,
                operator_id: self.operator_id,
                schema_version: SchemaVersion::V1,
                source,
                times,
                external_series_id: format!("{}:{}", self.id, event.provider_record_id),
                revision: 1,
                supersedes_id: None,
                status: WeatherHazardStatus::Active,
                issued_at: timestamp,
                provider_received_at: None,
                hazard_type: hazard_type.clone(),
                severity: *severity,
                valid_from: offset_time(self.start_time, *valid_from_offset_ms)?,
                valid_to: offset_time(self.start_time, *valid_to_offset_ms)?,
                altitude_band: *altitude_band,
                footprint: footprint.clone(),
            }),
        };

        Ok(NormalizedEventBatch {
            envelope,
            events: vec![canonical],
        })
    }
}

fn offset_time(start: DateTime<Utc>, offset_ms: u64) -> Result<DateTime<Utc>, ScenarioError> {
    let milliseconds = i64::try_from(offset_ms)
        .map_err(|_| ScenarioError::Validation("event offset exceeds supported duration".into()))?;
    start
        .checked_add_signed(Duration::milliseconds(milliseconds))
        .ok_or_else(|| ScenarioError::Validation("event time exceeds supported range".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ReplayScenario {
        ReplayScenario::from_json(include_str!(
            "../../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap()
    }

    #[test]
    fn milestone_fixture_covers_required_flight_roles() {
        let scenario = fixture();
        let roles: HashSet<_> = scenario.flights.iter().map(|flight| flight.role).collect();

        assert_eq!(roles.len(), 3);
        assert!(roles.contains(&FlightRole::Normal));
        assert!(roles.contains(&FlightRole::Delayed));
        assert!(roles.contains(&FlightRole::HazardAdjacent));
    }

    #[test]
    fn normalization_is_identical_for_the_same_fixture_event() {
        let scenario = fixture();
        assert_eq!(
            scenario.batch_for(&scenario.events[0]).unwrap(),
            scenario.batch_for(&scenario.events[0]).unwrap()
        );
    }
}
