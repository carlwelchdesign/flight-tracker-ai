use std::{convert::Infallible, time::Duration};

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::get,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, AuthFailure, Permission, require},
    domain::FlightId,
    metrics::ApiMetrics,
};

use super::{FleetEvent, FleetStore};

const DEFAULT_PAGE_SIZE: usize = 25;
const MAX_PAGE_SIZE: usize = 100;

#[derive(Clone)]
struct FleetHttpState {
    store: FleetStore,
    metrics: ApiMetrics,
}

#[derive(Debug, Deserialize)]
struct PaginationQuery {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_page_size")]
    page_size: usize,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

enum ApiError {
    InvalidPagination,
    InvalidFlightId,
    InvalidEventId,
    FlightNotFound,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidPagination => (
                StatusCode::BAD_REQUEST,
                "invalid_pagination",
                format!("page must be at least 1 and page_size must be 1 through {MAX_PAGE_SIZE}"),
            ),
            Self::InvalidFlightId => (
                StatusCode::BAD_REQUEST,
                "invalid_flight_id",
                "flight ID must be a UUID".into(),
            ),
            Self::InvalidEventId => (
                StatusCode::BAD_REQUEST,
                "invalid_last_event_id",
                "Last-Event-ID must be an unsigned integer".into(),
            ),
            Self::FlightNotFound => (
                StatusCode::NOT_FOUND,
                "flight_not_found",
                "flight was not found".into(),
            ),
        };
        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

pub fn fleet_router(store: FleetStore, metrics: ApiMetrics) -> Router {
    let state = FleetHttpState {
        store,
        metrics: metrics.clone(),
    };
    Router::new()
        .route("/api/flights", get(list_flights))
        .route("/api/flights/{flight_id}", get(flight_detail))
        .route("/api/flights/{flight_id}/timeline", get(flight_timeline))
        .route("/api/events/stream", get(event_stream))
        .route("/metrics", get(metrics_endpoint))
        .with_state(state)
}

async fn list_flights(
    State(state): State<FleetHttpState>,
    Extension(context): Extension<AuthContext>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    validate_pagination(&query).map_err(IntoResponse::into_response)?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(
            state
                .store
                .list(context.operator_id, query.page, query.page_size)
                .await,
        ),
    ))
}

async fn flight_detail(
    State(state): State<FleetHttpState>,
    Extension(context): Extension<AuthContext>,
    Path(flight_id): Path<String>,
) -> Result<impl IntoResponse, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    let flight_id = parse_flight_id(&flight_id).map_err(IntoResponse::into_response)?;
    state
        .store
        .detail(context.operator_id, flight_id)
        .await
        .map(|view| ([(header::CACHE_CONTROL, "no-store")], Json(view)))
        .ok_or_else(|| ApiError::FlightNotFound.into_response())
}

async fn flight_timeline(
    State(state): State<FleetHttpState>,
    Extension(context): Extension<AuthContext>,
    Path(flight_id): Path<String>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    validate_pagination(&query).map_err(IntoResponse::into_response)?;
    let flight_id = parse_flight_id(&flight_id).map_err(IntoResponse::into_response)?;
    state
        .store
        .timeline(context.operator_id, flight_id, query.page, query.page_size)
        .await
        .map(|timeline| ([(header::CACHE_CONTROL, "no-store")], Json(timeline)))
        .ok_or_else(|| ApiError::FlightNotFound.into_response())
}

async fn event_stream(
    State(state): State<FleetHttpState>,
    Extension(context): Extension<AuthContext>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>>, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    let mut last_sent = parse_last_event_id(&headers).map_err(IntoResponse::into_response)?;
    let mut receiver = state.store.subscribe();
    let operator_id = context.operator_id;
    let replay = state.store.events_after(operator_id, last_sent).await;
    let store = state.store.clone();
    let metrics = state.metrics.clone();
    let stream = async_stream::stream! {
        let _connection = metrics.stream_opened();
        for event in replay {
            if event.id > last_sent {
                last_sent = event.id;
                yield Ok(to_sse_event(&event));
            }
        }
        loop {
            match receiver.recv().await {
                Ok(event) if event.operator_id == operator_id && event.id > last_sent => {
                    last_sent = event.id;
                    yield Ok(to_sse_event(&event));
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    for event in store.events_after(operator_id, last_sent).await {
                        if event.id > last_sent {
                            last_sent = event.id;
                            yield Ok(to_sse_event(&event));
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

async fn metrics_endpoint(
    State(state): State<FleetHttpState>,
    Extension(context): Extension<AuthContext>,
) -> Result<impl IntoResponse, AuthFailure> {
    require(&context, Permission::ReadMetrics)?;
    Ok((
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.prometheus(),
    ))
}

fn validate_pagination(query: &PaginationQuery) -> Result<(), ApiError> {
    if query.page == 0 || query.page_size == 0 || query.page_size > MAX_PAGE_SIZE {
        Err(ApiError::InvalidPagination)
    } else {
        Ok(())
    }
}

fn parse_flight_id(value: &str) -> Result<FlightId, ApiError> {
    Uuid::parse_str(value)
        .map(FlightId::from_uuid)
        .map_err(|_| ApiError::InvalidFlightId)
}

fn parse_last_event_id(headers: &HeaderMap) -> Result<u64, ApiError> {
    match headers.get("last-event-id") {
        None => Ok(0),
        Some(value) => value
            .to_str()
            .ok()
            .and_then(|value| value.parse().ok())
            .ok_or(ApiError::InvalidEventId),
    }
}

fn to_sse_event(event: &FleetEvent) -> Event {
    Event::default()
        .id(event.id.to_string())
        .event("fleet_event")
        .json_data(event)
        .expect("fleet events serialize")
}

const fn default_page() -> usize {
    1
}

const fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::{body::Body, http::Request, middleware};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;
    use crate::{
        auth::{AuthContext, AuthRole},
        metrics::observe_request,
        replay::{ReplayScenario, ScenarioEvent},
    };

    fn fixture() -> ReplayScenario {
        ReplayScenario::from_json(include_str!(
            "../../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap()
    }

    async fn store_with_events(scenario: &ReplayScenario, events: &[ScenarioEvent]) -> FleetStore {
        let store = FleetStore::new(64);
        for event in events {
            store
                .apply(&scenario.batch_for(event).unwrap())
                .await
                .unwrap();
        }
        store
    }

    async fn json(response: Response) -> Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    fn authenticated_app(store: FleetStore, metrics: ApiMetrics) -> Router {
        fleet_router(store, metrics).layer(Extension(AuthContext {
            identity_id: Uuid::nil(),
            operator_id: fixture().operator_id,
            operator_code: "SIM".into(),
            operator_name: "Simulation Operator".into(),
            provider: "test".into(),
            subject: "test-user".into(),
            session_id: "test-session".into(),
            role: AuthRole::Administrator,
        }))
    }

    #[tokio::test]
    async fn list_detail_and_timeline_have_typed_paginated_contracts() {
        let scenario = fixture();
        let store = store_with_events(&scenario, &scenario.events).await;
        let app = authenticated_app(store, ApiMetrics::default());

        let list = app
            .clone()
            .oneshot(
                Request::get("/api/flights?page=1&page_size=2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list.status(), StatusCode::OK);
        assert_eq!(list.headers()[header::CACHE_CONTROL], "no-store");
        let payload = json(list).await;
        assert_eq!(payload["data"].as_array().unwrap().len(), 2);
        assert_eq!(payload["pagination"]["total_items"], 3);
        assert_eq!(payload["pagination"]["total_pages"], 2);

        let flight_id = scenario.flights[0].id.as_uuid();
        let detail = app
            .clone()
            .oneshot(
                Request::get(format!("/api/flights/{flight_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(detail.status(), StatusCode::OK);
        assert_eq!(detail.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(json(detail).await["flight"]["callsign"], "FT101");

        let timeline = app
            .oneshot(
                Request::get(format!(
                    "/api/flights/{flight_id}/timeline?page=1&page_size=10"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(timeline.status(), StatusCode::OK);
        assert_eq!(timeline.headers()[header::CACHE_CONTROL], "no-store");
        let payload = json(timeline).await;
        let events = payload["data"].as_array().unwrap();
        assert!(!events.is_empty());
        assert!(events.iter().all(|event| event["source"].is_object()));
    }

    #[tokio::test]
    async fn invalid_pagination_and_unknown_flights_return_structured_errors() {
        let app = authenticated_app(FleetStore::new(16), ApiMetrics::default());
        let invalid = app
            .clone()
            .oneshot(
                Request::get("/api/flights?page=0&page_size=101")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json(invalid).await["error"]["code"], "invalid_pagination");

        let missing = app
            .oneshot(
                Request::get(format!("/api/flights/{}", Uuid::nil()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert_eq!(json(missing).await["error"]["code"], "flight_not_found");
    }

    #[tokio::test]
    async fn sse_reconnect_replays_only_events_after_last_event_id() {
        let scenario = fixture();
        let store = store_with_events(
            &scenario,
            &[scenario.events[1].clone(), scenario.events[4].clone()],
        )
        .await;
        let app = authenticated_app(store, ApiMetrics::default());
        let response = app
            .oneshot(
                Request::get("/api/events/stream")
                    .header("last-event-id", "1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/event-stream"
        );

        let mut body = response.into_body();
        let frame = tokio::time::timeout(Duration::from_secs(1), body.frame())
            .await
            .expect("SSE replay should be immediate")
            .expect("SSE body should continue")
            .expect("SSE body should not error");
        let data = frame.into_data().expect("first SSE frame should be data");
        let text = std::str::from_utf8(&data).unwrap();
        assert!(text.contains("id: 2"));
        assert!(!text.contains("id: 1\n"));
        assert!(text.contains("event: fleet_event"));
    }

    #[tokio::test]
    async fn sse_rejects_an_invalid_last_event_id() {
        let response = authenticated_app(FleetStore::new(16), ApiMetrics::default())
            .oneshot(
                Request::get("/api/events/stream")
                    .header("last-event-id", "not-an-event-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            json(response).await["error"]["code"],
            "invalid_last_event_id"
        );
    }

    #[tokio::test]
    async fn metrics_endpoint_exposes_request_latency_and_stream_counters() {
        let metrics = ApiMetrics::default();
        let app = authenticated_app(FleetStore::new(16), metrics.clone())
            .layer(middleware::from_fn_with_state(metrics, observe_request));
        app.clone()
            .oneshot(Request::get("/api/flights").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let response = app
            .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(text.contains("flight_tracker_api_requests_total 1"));
        assert!(text.contains("flight_tracker_api_latency_microseconds_total"));
        assert!(text.contains("flight_tracker_stream_connections_active 0"));
    }
}
