use std::time::Duration;

use chrono::Utc;
use thiserror::Error;
use tokio::time::{MissedTickBehavior, interval};

use crate::{domain::OperatorId, health::WorkerProbe, ingestion::IngestionHub};

use super::{
    AdsbLolClient,
    adsb_lol::{AdsbLolError, normalize_snapshot},
    status::{LivePositionRegion, LivePositionStatusStore},
};

const MINIMUM_POLL_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct AdsbLolRuntimeConfig {
    pub operator_id: OperatorId,
    pub region: LivePositionRegion,
    pub initial_delay: Duration,
    pub poll_interval: Duration,
    pub stale_after: Duration,
}

#[derive(Debug, Error)]
pub enum AdsbLolRuntimeConfigError {
    #[error("ADSB.lol polling cannot run more frequently than once every 30 seconds")]
    PollIntervalTooShort,
    #[error("ADSB.lol radius must be between 1 and 100 nautical miles")]
    InvalidRadius,
    #[error("ADSB.lol center must use finite WGS84 latitude and longitude")]
    InvalidCenter,
}

impl AdsbLolRuntimeConfig {
    pub fn validate(&self) -> Result<(), AdsbLolRuntimeConfigError> {
        if self.poll_interval < MINIMUM_POLL_INTERVAL {
            return Err(AdsbLolRuntimeConfigError::PollIntervalTooShort);
        }
        if !(1..=100).contains(&self.region.radius_nautical_miles) {
            return Err(AdsbLolRuntimeConfigError::InvalidRadius);
        }
        if !self.region.latitude_degrees.is_finite()
            || !(-90.0..=90.0).contains(&self.region.latitude_degrees)
            || !self.region.longitude_degrees.is_finite()
            || !(-180.0..=180.0).contains(&self.region.longitude_degrees)
        {
            return Err(AdsbLolRuntimeConfigError::InvalidCenter);
        }
        Ok(())
    }
}

pub fn spawn_adsb_lol_runtime(
    client: AdsbLolClient,
    ingestion: IngestionHub,
    statuses: LivePositionStatusStore,
    config: AdsbLolRuntimeConfig,
    probe: WorkerProbe,
) -> Result<(), AdsbLolRuntimeConfigError> {
    config.validate()?;
    statuses.register(
        config.operator_id,
        config.region,
        config.stale_after,
        Utc::now(),
    );
    tokio::spawn(run_runtime(client, ingestion, statuses, config, probe));
    Ok(())
}

async fn run_runtime(
    client: AdsbLolClient,
    ingestion: IngestionHub,
    statuses: LivePositionStatusStore,
    config: AdsbLolRuntimeConfig,
    probe: WorkerProbe,
) {
    tokio::time::sleep(config.initial_delay).await;
    let mut poll = interval(config.poll_interval);
    poll.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        poll.tick().await;
        probe.heartbeat();
        process_poll(
            client.fetch_point(config.region).await,
            &ingestion,
            &statuses,
            &config,
        );
        probe.heartbeat();
    }
}

fn process_poll(
    result: Result<super::adsb_lol::AdsbLolPayload, AdsbLolError>,
    ingestion: &IngestionHub,
    statuses: &LivePositionStatusStore,
    config: &AdsbLolRuntimeConfig,
) {
    let now = Utc::now();
    match result.and_then(|payload| {
        normalize_snapshot(payload, config.operator_id, now, config.stale_after)
    }) {
        Ok(snapshot) => {
            let published_batches = snapshot.batches.len();
            for batch in snapshot.batches {
                ingestion.publish(batch);
            }
            statuses.record_success(
                config.operator_id,
                now,
                snapshot.newest_position_at,
                snapshot.coverage,
            );
            tracing::info!(
                provider = "adsb.lol",
                feed = "point",
                published_batches,
                rejected_records = snapshot.coverage.rejected_record_count,
                "best-effort live position snapshot processed"
            );
        }
        Err(error) => {
            statuses.record_failure(config.operator_id, now, error.code());
            tracing::warn!(
                provider = "adsb.lol",
                feed = "point",
                error_code = error.code(),
                "best-effort live position poll failed; replay remains available"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_enforces_region_and_polling_bounds() {
        let base = AdsbLolRuntimeConfig {
            operator_id: OperatorId::new(),
            region: LivePositionRegion {
                latitude_degrees: 37.62,
                longitude_degrees: -122.38,
                radius_nautical_miles: 25,
            },
            initial_delay: Duration::ZERO,
            poll_interval: Duration::from_secs(30),
            stale_after: Duration::from_secs(30),
        };
        assert!(base.validate().is_ok());
        assert!(matches!(
            AdsbLolRuntimeConfig {
                poll_interval: Duration::from_secs(29),
                ..base.clone()
            }
            .validate(),
            Err(AdsbLolRuntimeConfigError::PollIntervalTooShort)
        ));
        assert!(matches!(
            AdsbLolRuntimeConfig {
                region: LivePositionRegion {
                    radius_nautical_miles: 101,
                    ..base.region
                },
                ..base.clone()
            }
            .validate(),
            Err(AdsbLolRuntimeConfigError::InvalidRadius)
        ));
        assert!(matches!(
            AdsbLolRuntimeConfig {
                region: LivePositionRegion {
                    latitude_degrees: 91.0,
                    ..base.region
                },
                ..base
            }
            .validate(),
            Err(AdsbLolRuntimeConfigError::InvalidCenter)
        ));
    }
}
