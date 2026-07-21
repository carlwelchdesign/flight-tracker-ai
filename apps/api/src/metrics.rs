use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::observability::CorrelationId;

#[derive(Default)]
struct MetricsInner {
    api_requests_total: AtomicU64,
    api_latency_micros_total: AtomicU64,
    stream_connections_total: AtomicU64,
    stream_connections_active: AtomicU64,
}

#[derive(Clone, Default)]
pub struct ApiMetrics {
    inner: Arc<MetricsInner>,
}

impl ApiMetrics {
    pub fn prometheus(&self) -> String {
        format!(
            concat!(
                "# TYPE flight_tracker_api_requests_total counter\n",
                "flight_tracker_api_requests_total {}\n",
                "# TYPE flight_tracker_api_latency_microseconds_total counter\n",
                "flight_tracker_api_latency_microseconds_total {}\n",
                "# TYPE flight_tracker_stream_connections_total counter\n",
                "flight_tracker_stream_connections_total {}\n",
                "# TYPE flight_tracker_stream_connections_active gauge\n",
                "flight_tracker_stream_connections_active {}\n"
            ),
            self.inner.api_requests_total.load(Ordering::Relaxed),
            self.inner.api_latency_micros_total.load(Ordering::Relaxed),
            self.inner.stream_connections_total.load(Ordering::Relaxed),
            self.inner.stream_connections_active.load(Ordering::Relaxed),
        )
    }

    pub fn stream_opened(&self) -> StreamConnectionGuard {
        self.inner
            .stream_connections_total
            .fetch_add(1, Ordering::Relaxed);
        let active = self
            .inner
            .stream_connections_active
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        tracing::info!(active_stream_connections = active, "SSE stream opened");
        StreamConnectionGuard {
            metrics: self.clone(),
        }
    }
}

pub struct StreamConnectionGuard {
    metrics: ApiMetrics,
}

impl Drop for StreamConnectionGuard {
    fn drop(&mut self) {
        let active = self
            .metrics
            .inner
            .stream_connections_active
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1);
        tracing::info!(active_stream_connections = active, "SSE stream closed");
    }
}

pub async fn observe_request(
    State(metrics): State<ApiMetrics>,
    request: Request,
    next: Next,
) -> Response<Body> {
    let path = request.uri().path().to_owned();
    let method = request.method().clone();
    let correlation_id = request
        .extensions()
        .get::<CorrelationId>()
        .map(|value| value.as_str().to_owned())
        .unwrap_or_else(|| "missing".to_owned());
    let started = Instant::now();
    let response = next.run(request).await;
    let elapsed = started.elapsed();
    metrics
        .inner
        .api_requests_total
        .fetch_add(1, Ordering::Relaxed);
    metrics.inner.api_latency_micros_total.fetch_add(
        u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    tracing::info!(
        %correlation_id,
        %method,
        %path,
        status = response.status().as_u16(),
        latency_micros = elapsed.as_micros(),
        "API request completed"
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_guard_tracks_open_and_closed_connections() {
        let metrics = ApiMetrics::default();
        let guard = metrics.stream_opened();
        assert!(
            metrics
                .prometheus()
                .contains("flight_tracker_stream_connections_active 1")
        );
        drop(guard);
        let output = metrics.prometheus();
        assert!(output.contains("flight_tracker_stream_connections_total 1"));
        assert!(output.contains("flight_tracker_stream_connections_active 0"));
    }
}
