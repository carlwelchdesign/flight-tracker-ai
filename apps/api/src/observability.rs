use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::Instrument;
use uuid::Uuid;

pub const CORRELATION_HEADER: HeaderName = HeaderName::from_static("x-correlation-id");

#[derive(Debug, Clone)]
pub struct CorrelationId(String);

impl CorrelationId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub async fn correlate_request(mut request: Request, next: Next) -> Response {
    let correlation_id = request
        .headers()
        .get(&CORRELATION_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| valid_correlation_id(value))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    request
        .extensions_mut()
        .insert(CorrelationId(correlation_id.clone()));

    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let span = tracing::info_span!(
        "http.request",
        correlation_id = %correlation_id,
        %method,
        %path
    );
    let mut response = next.run(request).instrument(span).await;
    response.headers_mut().insert(
        CORRELATION_HEADER,
        HeaderValue::from_str(&correlation_id).expect("validated correlation ID is a header value"),
    );
    response
}

fn valid_correlation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
    };

    use axum::{Router, body::Body, http::Request, middleware, routing::get};
    use serde_json::Value;
    use tower::ServiceExt;
    use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt};

    use super::*;
    use crate::metrics::{ApiMetrics, observe_request};

    #[tokio::test]
    async fn preserves_valid_correlation_ids() {
        let response = app()
            .oneshot(
                Request::get("/")
                    .header(&CORRELATION_HEADER, "dispatch-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.headers().get(&CORRELATION_HEADER).unwrap(),
            "dispatch-123"
        );
    }

    #[tokio::test]
    async fn replaces_unsafe_correlation_ids() {
        let response = app()
            .oneshot(
                Request::get("/")
                    .header(&CORRELATION_HEADER, "spaces are rejected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let generated = response
            .headers()
            .get(&CORRELATION_HEADER)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(Uuid::parse_str(generated).is_ok());
    }

    #[test]
    fn structured_events_are_json_with_the_correlation_id() {
        let writer = TestWriter::default();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(writer.clone()),
        );
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(
                correlation_id = "operator-check-001",
                status = 200,
                "API request completed"
            );
        });

        let entries = writer.entries();
        let request_log = entries
            .iter()
            .find(|entry| entry["fields"]["message"] == "API request completed")
            .unwrap_or_else(|| panic!("request completion log should be emitted: {entries:?}"));
        assert_eq!(
            request_log["fields"]["correlation_id"],
            "operator-check-001"
        );
        assert_eq!(request_log["fields"]["status"], 200);
    }

    fn app() -> Router {
        let metrics = ApiMetrics::default();
        Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(metrics, observe_request))
            .layer(middleware::from_fn(correlate_request))
    }

    #[derive(Clone, Default)]
    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl TestWriter {
        fn entries(&self) -> Vec<Value> {
            let output = self.0.lock().unwrap();
            std::str::from_utf8(&output)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        }
    }

    impl<'a> MakeWriter<'a> for TestWriter {
        type Writer = TestWriterGuard;

        fn make_writer(&'a self) -> Self::Writer {
            TestWriterGuard(self.0.clone())
        }
    }

    struct TestWriterGuard(Arc<Mutex<Vec<u8>>>);

    impl Write for TestWriterGuard {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
