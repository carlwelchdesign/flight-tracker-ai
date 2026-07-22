use std::time::Duration;

use chrono::Utc;
use thiserror::Error;
use tokio::time::MissedTickBehavior;

use crate::{
    domain::OperatorId,
    health::{WorkerProbe, maintain_worker_heartbeat},
    ingestion::IngestionHub,
};

use super::{
    AdsbLolClient,
    adsb_lol::{AdsbLolError, AdsbLolPayload, normalize_snapshot},
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
    #[error("live-position polling cannot run more frequently than once every 30 seconds")]
    PollIntervalTooShort,
    #[error("live-position radius must be between 1 and 100 nautical miles")]
    InvalidRadius,
    #[error("live-position center must use finite WGS84 latitude and longitude")]
    InvalidCenter,
}

#[derive(Clone)]
pub struct LivePositionClientChain {
    primary: AdsbLolClient,
    fallback: Option<AdsbLolClient>,
}

impl LivePositionClientChain {
    pub fn new(primary: AdsbLolClient, fallback: Option<AdsbLolClient>) -> Self {
        Self { primary, fallback }
    }

    async fn fetch_point(
        &self,
        region: LivePositionRegion,
    ) -> Result<AdsbLolPayload, LivePositionFetchError> {
        match self.primary.fetch_point(region).await {
            Ok(payload) => Ok(payload),
            Err(primary) => {
                let Some(fallback) = &self.fallback else {
                    return Err(LivePositionFetchError::Primary(primary));
                };
                tracing::warn!(
                    provider = "adsb.lol",
                    fallback_provider = "airplanes.live",
                    error_code = primary.code(),
                    "primary live position request failed; attempting portfolio fallback"
                );
                fallback
                    .fetch_point(region)
                    .await
                    .map_err(|fallback| LivePositionFetchError::Both { primary, fallback })
            }
        }
    }
}

#[derive(Debug, Error)]
enum LivePositionFetchError {
    #[error("primary live-position provider failed: {0}")]
    Primary(AdsbLolError),
    #[error("primary provider failed ({primary}); fallback provider failed ({fallback})")]
    Both {
        primary: AdsbLolError,
        fallback: AdsbLolError,
    },
}

impl LivePositionFetchError {
    fn code(&self) -> &'static str {
        match self {
            Self::Primary(error) => error.code(),
            Self::Both { .. } => "all_providers_unavailable",
        }
    }
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
    clients: LivePositionClientChain,
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
    tokio::spawn(run_runtime(clients, ingestion, statuses, config, probe));
    Ok(())
}

async fn run_runtime(
    clients: LivePositionClientChain,
    ingestion: IngestionHub,
    statuses: LivePositionStatusStore,
    config: AdsbLolRuntimeConfig,
    probe: WorkerProbe,
) {
    let mut poll = tokio::time::interval_at(
        tokio::time::Instant::now() + config.initial_delay,
        config.poll_interval,
    );
    poll.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        maintain_worker_heartbeat(&probe, async {
            poll.tick().await;
            process_poll(
                clients.fetch_point(config.region).await,
                &ingestion,
                &statuses,
                &config,
            );
        })
        .await;
    }
}

fn process_poll(
    result: Result<AdsbLolPayload, LivePositionFetchError>,
    ingestion: &IngestionHub,
    statuses: &LivePositionStatusStore,
    config: &AdsbLolRuntimeConfig,
) {
    let now = Utc::now();
    match result {
        Ok(payload) => {
            let provider = payload.provider;
            match normalize_snapshot(payload, config.operator_id, now, config.stale_after) {
                Ok(snapshot) => {
                    let published_batches = snapshot.batches.len();
                    for batch in snapshot.batches {
                        ingestion.publish(batch);
                    }
                    statuses.record_success(
                        config.operator_id,
                        provider,
                        now,
                        snapshot.newest_position_at,
                        snapshot.coverage,
                    );
                    tracing::info!(
                        provider = provider.id(),
                        feed = "point",
                        published_batches,
                        rejected_records = snapshot.coverage.rejected_record_count,
                        "best-effort live position snapshot processed"
                    );
                }
                Err(error) => {
                    statuses.record_failure(config.operator_id, now, error.code());
                    tracing::warn!(
                        provider = provider.id(),
                        feed = "point",
                        error_code = error.code(),
                        "best-effort live position poll failed; replay remains available"
                    );
                }
            }
        }
        Err(error) => {
            statuses.record_failure(config.operator_id, now, error.code());
            tracing::warn!(
                provider = "live-position-chain",
                feed = "point",
                error_code = error.code(),
                "best-effort live position poll failed; replay remains available"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use axum::{Json, Router, http::StatusCode, routing::get};
    use reqwest::Url;
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;
    use crate::live_positions::{AdsbLolClientConfig, LivePositionProvider, RetryPolicy};

    async fn client_for(
        provider: LivePositionProvider,
        router: Router,
        minimum_request_interval: Option<Duration>,
    ) -> AdsbLolClient {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        AdsbLolClient::new(AdsbLolClientConfig {
            provider,
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            user_agent: "flight-tracker-ai-test/1.0".into(),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            retry: RetryPolicy {
                max_attempts: 1,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
            },
            minimum_request_interval,
        })
        .unwrap()
    }

    fn region() -> LivePositionRegion {
        LivePositionRegion {
            latitude_degrees: 37.62,
            longitude_degrees: -122.38,
            radius_nautical_miles: 25,
        }
    }

    #[tokio::test]
    async fn primary_success_never_calls_fallback() {
        let fallback_attempts = Arc::new(AtomicUsize::new(0));
        let attempts = fallback_attempts.clone();
        let primary = client_for(
            LivePositionProvider::AdsbLol,
            Router::new().route(
                "/v2/point/37.62/-122.38/25",
                get(|| async { Json(json!({ "now": 1784654160000_i64, "ac": [] })) }),
            ),
            None,
        )
        .await;
        let fallback = client_for(
            LivePositionProvider::AirplanesLive,
            Router::new().route(
                "/v2/point/37.62/-122.38/25",
                get(move || {
                    let attempts = attempts.clone();
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Json(json!({ "now": 1784654160000_i64, "ac": [] }))
                    }
                }),
            ),
            Some(Duration::from_millis(10)),
        )
        .await;

        let payload = LivePositionClientChain::new(primary, Some(fallback))
            .fetch_point(region())
            .await
            .unwrap();

        assert_eq!(payload.provider, LivePositionProvider::AdsbLol);
        assert_eq!(fallback_attempts.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn primary_failure_uses_fallback_and_both_failures_are_explicit() {
        let unavailable = || {
            Router::new().route(
                "/v2/point/37.62/-122.38/25",
                get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
            )
        };
        let primary = client_for(LivePositionProvider::AdsbLol, unavailable(), None).await;
        let fallback = client_for(
            LivePositionProvider::AirplanesLive,
            Router::new().route(
                "/v2/point/37.62/-122.38/25",
                get(|| async { Json(json!({ "now": 1784654160000_i64, "ac": [] })) }),
            ),
            Some(Duration::from_millis(10)),
        )
        .await;
        let payload = LivePositionClientChain::new(primary, Some(fallback))
            .fetch_point(region())
            .await
            .unwrap();
        assert_eq!(payload.provider, LivePositionProvider::AirplanesLive);

        let primary = client_for(LivePositionProvider::AdsbLol, unavailable(), None).await;
        let fallback = client_for(
            LivePositionProvider::AirplanesLive,
            unavailable(),
            Some(Duration::from_millis(10)),
        )
        .await;
        let error = LivePositionClientChain::new(primary, Some(fallback))
            .fetch_point(region())
            .await
            .unwrap_err();
        assert_eq!(error.code(), "all_providers_unavailable");
        assert!(matches!(error, LivePositionFetchError::Both { .. }));
    }

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
