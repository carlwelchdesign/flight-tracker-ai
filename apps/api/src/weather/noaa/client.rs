use std::time::Duration;

use rand::RngExt;
use reqwest::{StatusCode, Url, header};
use serde_json::Value;
use thiserror::Error;

const METAR_PATH: &str = "api/data/metar";
const AIRSIGMET_PATH: &str = "api/data/airsigmet";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoaaFeed {
    Metar,
    AirSigmet,
}

impl NoaaFeed {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Metar => "metar",
            Self::AirSigmet => "airsigmet",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoaaPayload {
    pub feed: NoaaFeed,
    pub value: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(2),
        }
    }
}

impl RetryPolicy {
    fn delay_with_jitter(&self, retry_number: u32, jitter_fraction: f64) -> Duration {
        let exponent = retry_number.saturating_sub(1).min(31);
        let cap = self
            .base_delay
            .saturating_mul(1_u32 << exponent)
            .min(self.max_delay);
        cap.mul_f64(jitter_fraction.clamp(0.0, 1.0))
    }
}

#[derive(Debug, Clone)]
pub struct NoaaClientConfig {
    pub base_url: Url,
    pub user_agent: String,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub retry: RetryPolicy,
}

#[derive(Debug, Error)]
pub enum NoaaClientError {
    #[error("NOAA request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("NOAA returned HTTP {status}: {body}")]
    Http { status: StatusCode, body: String },
    #[error("NOAA returned malformed JSON: {0}")]
    MalformedJson(serde_json::Error),
    #[error("NOAA client configuration is invalid: {0}")]
    Configuration(String),
}

impl NoaaClientError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request(error) if error.is_timeout() => "timeout",
            Self::Request(_) => "request_failed",
            Self::Http { status, .. } if *status == StatusCode::TOO_MANY_REQUESTS => "rate_limited",
            Self::Http { status, .. } if status.is_server_error() => "provider_unavailable",
            Self::Http { .. } => "invalid_request",
            Self::MalformedJson(_) => "malformed_json",
            Self::Configuration(_) => "configuration",
        }
    }
}

#[derive(Clone)]
pub struct NoaaClient {
    client: reqwest::Client,
    base_url: Url,
    retry: RetryPolicy,
}

impl NoaaClient {
    pub fn new(config: NoaaClientConfig) -> Result<Self, NoaaClientError> {
        if config.user_agent.trim().is_empty() {
            return Err(NoaaClientError::Configuration(
                "user agent must identify the consumer".into(),
            ));
        }
        if !matches!(config.base_url.scheme(), "http" | "https") {
            return Err(NoaaClientError::Configuration(
                "base URL must use HTTP or HTTPS".into(),
            ));
        }
        if config.retry.max_attempts == 0 {
            return Err(NoaaClientError::Configuration(
                "max_attempts must be greater than zero".into(),
            ));
        }
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        let client = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .user_agent(config.user_agent)
            .default_headers(headers)
            .build()?;
        Ok(Self {
            client,
            base_url: config.base_url,
            retry: config.retry,
        })
    }

    pub async fn fetch_metars(&self, stations: &[String]) -> Result<NoaaPayload, NoaaClientError> {
        let station_list = stations.join(",");
        self.fetch(
            NoaaFeed::Metar,
            METAR_PATH,
            &[
                ("ids", station_list.as_str()),
                ("format", "json"),
                ("hours", "2"),
            ],
        )
        .await
    }

    pub async fn fetch_air_sigmets(&self) -> Result<NoaaPayload, NoaaClientError> {
        self.fetch(NoaaFeed::AirSigmet, AIRSIGMET_PATH, &[("format", "json")])
            .await
    }

    async fn fetch(
        &self,
        feed: NoaaFeed,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<NoaaPayload, NoaaClientError> {
        let url = self
            .base_url
            .join(path)
            .map_err(|error| NoaaClientError::Configuration(error.to_string()))?;
        let mut attempt = 1;
        loop {
            let result = self.client.get(url.clone()).query(query).send().await;
            match result {
                Ok(response) if response.status() == StatusCode::NO_CONTENT => {
                    return Ok(NoaaPayload { feed, value: None });
                }
                Ok(response) if response.status().is_success() => {
                    let bytes = response.bytes().await?;
                    let value =
                        serde_json::from_slice(&bytes).map_err(NoaaClientError::MalformedJson)?;
                    return Ok(NoaaPayload {
                        feed,
                        value: Some(value),
                    });
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    let error = NoaaClientError::Http { status, body };
                    if attempt >= self.retry.max_attempts || !is_retryable_status(status) {
                        return Err(error);
                    }
                }
                Err(error) => {
                    let retryable = error.is_timeout() || error.is_connect();
                    if attempt >= self.retry.max_attempts || !retryable {
                        return Err(NoaaClientError::Request(error));
                    }
                }
            }
            let jitter = rand::rng().random_range(0.0..=1.0);
            tokio::time::sleep(self.retry.delay_with_jitter(attempt, jitter)).await;
            attempt += 1;
        }
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || matches!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
        )
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use axum::{
        Json, Router, extract::State, http::HeaderMap, response::IntoResponse, routing::get,
    };
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn exponential_backoff_is_bounded_and_jittered() {
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(250),
        };
        assert_eq!(policy.delay_with_jitter(1, 0.0), Duration::ZERO);
        assert_eq!(policy.delay_with_jitter(1, 1.0), Duration::from_millis(100));
        assert_eq!(policy.delay_with_jitter(2, 1.0), Duration::from_millis(200));
        assert_eq!(policy.delay_with_jitter(3, 1.0), Duration::from_millis(250));
    }

    #[test]
    fn user_agent_is_required() {
        let error = NoaaClient::new(NoaaClientConfig {
            base_url: Url::parse("https://example.invalid/").unwrap(),
            user_agent: " ".into(),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            retry: RetryPolicy::default(),
        })
        .err()
        .unwrap();
        assert!(matches!(error, NoaaClientError::Configuration(_)));
    }

    #[derive(Clone, Default)]
    struct TestState {
        attempts: Arc<AtomicUsize>,
        user_agent: Arc<Mutex<Option<String>>>,
    }

    #[tokio::test]
    async fn retryable_responses_back_off_and_preserve_identifiable_user_agent() {
        async fn metar(State(state): State<TestState>, headers: HeaderMap) -> impl IntoResponse {
            *state.user_agent.lock().unwrap() = headers
                .get(header::USER_AGENT)
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned);
            if state.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                return StatusCode::TOO_MANY_REQUESTS.into_response();
            }
            Json(json!([])).into_response()
        }

        let state = TestState::default();
        let router = Router::new()
            .route("/api/data/metar", get(metar))
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = NoaaClient::new(NoaaClientConfig {
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            user_agent: "flight-tracker-ai-test/1.0".into(),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            retry: RetryPolicy {
                max_attempts: 2,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
            },
        })
        .unwrap();
        let payload = client.fetch_metars(&["KSFO".into()]).await.unwrap();
        assert_eq!(payload.value, Some(json!([])));
        assert_eq!(state.attempts.load(Ordering::SeqCst), 2);
        assert_eq!(
            state.user_agent.lock().unwrap().as_deref(),
            Some("flight-tracker-ai-test/1.0")
        );
    }

    #[tokio::test]
    async fn no_content_is_a_successful_empty_feed() {
        let router = Router::new().route(
            "/api/data/airsigmet",
            get(|| async { StatusCode::NO_CONTENT }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = NoaaClient::new(NoaaClientConfig {
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            user_agent: "flight-tracker-ai-test/1.0".into(),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            retry: RetryPolicy::default(),
        })
        .unwrap();
        assert_eq!(client.fetch_air_sigmets().await.unwrap().value, None);
    }

    #[tokio::test]
    async fn request_timeout_is_bounded_and_classified() {
        let router = Router::new().route(
            "/api/data/airsigmet",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Json(json!([]))
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = NoaaClient::new(NoaaClientConfig {
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            user_agent: "flight-tracker-ai-test/1.0".into(),
            connect_timeout: Duration::from_millis(10),
            request_timeout: Duration::from_millis(5),
            retry: RetryPolicy {
                max_attempts: 1,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
            },
        })
        .unwrap();
        let error = client.fetch_air_sigmets().await.unwrap_err();
        assert_eq!(error.code(), "timeout");
    }
}
