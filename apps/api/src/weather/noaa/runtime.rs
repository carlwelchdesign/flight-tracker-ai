use std::time::Duration;

use chrono::{DateTime, Utc};
use thiserror::Error;
use tokio::time::{MissedTickBehavior, interval};
use uuid::Uuid;

use crate::{
    domain::{OperatorId, SchemaVersion, SourceHealth, SourceHealthId, SourceHealthState},
    health::{WorkerProbe, maintain_worker_heartbeat},
    ingestion::IngestionHub,
};

use super::{
    NoaaClient, NoaaClientError, NoaaFeed, NoaaPayload, NoaaStore, PersistedNoaaRecord,
    prepare_records,
};

const SOURCE_HEALTH_NAMESPACE: Uuid = Uuid::from_u128(0x617838cb_98ee_5c61_91b6_2d7f6a548b08);
const PROVIDER: &str = "noaa-awc";
const MINIMUM_POLL_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct NoaaRuntimeConfig {
    pub operator_id: OperatorId,
    pub stations: Vec<String>,
    pub poll_interval: Duration,
    pub metar_stale_after: Duration,
    pub air_sigmet_stale_after: Duration,
}

#[derive(Debug, Error)]
pub enum NoaaRuntimeConfigError {
    #[error("NOAA polling cannot run more frequently than once per minute per endpoint")]
    PollIntervalTooShort,
    #[error("at least one four-character ICAO station is required")]
    MissingStations,
    #[error("invalid ICAO station code: {0}")]
    InvalidStation(String),
}

impl NoaaRuntimeConfig {
    pub fn validate(&self) -> Result<(), NoaaRuntimeConfigError> {
        if self.poll_interval < MINIMUM_POLL_INTERVAL {
            return Err(NoaaRuntimeConfigError::PollIntervalTooShort);
        }
        if self.stations.is_empty() {
            return Err(NoaaRuntimeConfigError::MissingStations);
        }
        if let Some(station) = self.stations.iter().find(|station| {
            station.len() != 4
                || !station
                    .chars()
                    .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
        }) {
            return Err(NoaaRuntimeConfigError::InvalidStation(station.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SourceHealthTracker {
    operator_id: OperatorId,
    feed: NoaaFeed,
    stale_after: Duration,
    evaluate_product_age: bool,
    last_success_at: Option<DateTime<Utc>>,
    newest_event_at: Option<DateTime<Utc>>,
    consecutive_failures: u32,
}

impl SourceHealthTracker {
    pub fn new(operator_id: OperatorId, feed: NoaaFeed, stale_after: Duration) -> Self {
        Self {
            operator_id,
            feed,
            stale_after,
            evaluate_product_age: feed == NoaaFeed::Metar,
            last_success_at: None,
            newest_event_at: None,
            consecutive_failures: 0,
        }
    }

    pub fn success(
        &mut self,
        now: DateTime<Utc>,
        newest_event_at: Option<DateTime<Utc>>,
    ) -> SourceHealth {
        self.last_success_at = Some(now);
        self.newest_event_at = newest_event_at.or(self.newest_event_at);
        self.consecutive_failures = 0;
        self.snapshot(now, None)
    }

    pub fn failure(&mut self, now: DateTime<Utc>, code: &str) -> SourceHealth {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.snapshot(now, Some(code.to_owned()))
    }

    fn snapshot(&self, now: DateTime<Utc>, last_error_code: Option<String>) -> SourceHealth {
        let product_delay = self
            .newest_event_at
            .and_then(|event| now.signed_duration_since(event).to_std().ok());
        let transport_delay = self
            .last_success_at
            .and_then(|success| now.signed_duration_since(success).to_std().ok());
        let delay = product_delay.or(transport_delay);
        let state = match self.consecutive_failures {
            0 if self.evaluate_product_age
                && product_delay.is_some_and(|delay| delay > self.stale_after) =>
            {
                SourceHealthState::Stale
            }
            0 => SourceHealthState::Healthy,
            1 => SourceHealthState::Unknown,
            2 => SourceHealthState::Stale,
            _ => SourceHealthState::Degraded,
        };
        let identity = format!(
            "{}:{PROVIDER}:{}",
            self.operator_id.as_uuid(),
            self.feed.as_str()
        );
        SourceHealth {
            id: SourceHealthId::from_uuid(Uuid::new_v5(
                &SOURCE_HEALTH_NAMESPACE,
                identity.as_bytes(),
            )),
            operator_id: self.operator_id,
            schema_version: SchemaVersion::V1,
            provider: PROVIDER.into(),
            feed: self.feed.as_str().into(),
            state,
            observed_at: now,
            last_attempt_at: now,
            last_success_at: self.last_success_at,
            newest_event_at: self.newest_event_at,
            consecutive_failures: self.consecutive_failures,
            delay_seconds: delay.map(|delay| delay.as_secs()),
            stale_after_seconds: self.stale_after.as_secs(),
            last_error_code,
        }
    }
}

pub fn spawn_noaa_runtime(
    client: NoaaClient,
    store: NoaaStore,
    ingestion: IngestionHub,
    config: NoaaRuntimeConfig,
    probe: WorkerProbe,
) -> Result<(), NoaaRuntimeConfigError> {
    config.validate()?;
    tokio::spawn(run_noaa_runtime(client, store, ingestion, config, probe));
    Ok(())
}

async fn run_noaa_runtime(
    client: NoaaClient,
    store: NoaaStore,
    ingestion: IngestionHub,
    config: NoaaRuntimeConfig,
    probe: WorkerProbe,
) {
    let mut metar_health = SourceHealthTracker::new(
        config.operator_id,
        NoaaFeed::Metar,
        config.metar_stale_after,
    );
    let mut sigmet_health = SourceHealthTracker::new(
        config.operator_id,
        NoaaFeed::AirSigmet,
        config.air_sigmet_stale_after,
    );
    let mut poll = interval(config.poll_interval);
    poll.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        maintain_worker_heartbeat(&probe, async {
            poll.tick().await;
            let metars = client.fetch_metars(&config.stations);
            let sigmets = client.fetch_air_sigmets();
            let (metar_result, sigmet_result) = tokio::join!(metars, sigmets);
            process_feed_result(
                metar_result,
                config.operator_id,
                &store,
                &ingestion,
                &mut metar_health,
            )
            .await;
            process_feed_result(
                sigmet_result,
                config.operator_id,
                &store,
                &ingestion,
                &mut sigmet_health,
            )
            .await;
        })
        .await;
    }
}

async fn process_feed_result(
    result: Result<NoaaPayload, NoaaClientError>,
    operator_id: OperatorId,
    store: &NoaaStore,
    ingestion: &IngestionHub,
    health: &mut SourceHealthTracker,
) {
    let now = Utc::now();
    let health_snapshot = match result {
        Ok(payload) => {
            let records = prepare_records(payload, operator_id, now);
            let newest_event_at = records
                .iter()
                .filter_map(|record| record.fact.as_ref().ok())
                .map(|fact| fact.event_time())
                .max();
            let mut quarantined = false;
            for record in records {
                let correlation_id = record.envelope.id.as_uuid();
                match store.persist_record(record).await {
                    Ok(PersistedNoaaRecord::Applied(batch)) => {
                        ingestion.publish(*batch);
                    }
                    Ok(PersistedNoaaRecord::Duplicate) => {}
                    Ok(PersistedNoaaRecord::Quarantined { code }) => {
                        quarantined = true;
                        tracing::warn!(
                            correlation_id = %correlation_id,
                            feed = health.feed.as_str(),
                            error_code = code,
                            "NOAA record quarantined"
                        );
                    }
                    Err(error) => {
                        tracing::error!(correlation_id = %correlation_id, feed = health.feed.as_str(), error = %error, "NOAA record persistence failed");
                        quarantined = true;
                    }
                }
            }
            if quarantined {
                health.failure(now, "record_quarantined")
            } else {
                health.success(now, newest_event_at)
            }
        }
        Err(error) => {
            tracing::warn!(correlation_id = %format_args!("noaa:{}", health.feed.as_str()), feed = health.feed.as_str(), error_code = error.code(), error = %error, "NOAA request failed");
            health.failure(now, error.code())
        }
    };
    if let Err(error) = store.upsert_source_health(&health_snapshot).await {
        tracing::error!(correlation_id = %format_args!("noaa:{}", health.feed.as_str()), feed = health.feed.as_str(), error = %error, "NOAA source health persistence failed");
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 21, 6, 0, 0).unwrap()
    }

    #[test]
    fn rate_discipline_requires_minute_intervals_and_valid_stations() {
        let operator_id = OperatorId::from_uuid(Uuid::nil());
        let mut config = NoaaRuntimeConfig {
            operator_id,
            stations: vec!["KSFO".into()],
            poll_interval: Duration::from_secs(59),
            metar_stale_after: Duration::from_secs(900),
            air_sigmet_stale_after: Duration::from_secs(180),
        };
        assert!(matches!(
            config.validate(),
            Err(NoaaRuntimeConfigError::PollIntervalTooShort)
        ));
        config.poll_interval = Duration::from_secs(60);
        assert!(config.validate().is_ok());
        config.stations = vec!["sfo".into()];
        assert!(matches!(
            config.validate(),
            Err(NoaaRuntimeConfigError::InvalidStation(_))
        ));
    }

    #[test]
    fn source_health_fails_closed_at_documented_thresholds() {
        let mut tracker = SourceHealthTracker::new(
            OperatorId::from_uuid(Uuid::nil()),
            NoaaFeed::Metar,
            Duration::from_secs(900),
        );
        let current = now();
        assert_eq!(
            tracker
                .success(current, Some(current - chrono::Duration::minutes(16)))
                .state,
            SourceHealthState::Stale
        );
        assert_eq!(
            tracker.failure(current, "timeout").state,
            SourceHealthState::Unknown
        );
        assert_eq!(
            tracker.failure(current, "timeout").state,
            SourceHealthState::Stale
        );
        let degraded = tracker.failure(current, "timeout");
        assert_eq!(degraded.state, SourceHealthState::Degraded);
        assert_eq!(degraded.consecutive_failures, 3);
    }

    #[test]
    fn successful_empty_feed_is_transport_healthy() {
        let mut tracker = SourceHealthTracker::new(
            OperatorId::from_uuid(Uuid::nil()),
            NoaaFeed::AirSigmet,
            Duration::from_secs(180),
        );
        assert_eq!(
            tracker.success(now(), None).state,
            SourceHealthState::Healthy
        );
        assert_eq!(
            tracker
                .success(now(), Some(now() - chrono::Duration::hours(1)))
                .state,
            SourceHealthState::Healthy
        );
    }
}
