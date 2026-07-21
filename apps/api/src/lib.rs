use axum::{Json, Router, extract::State, http::StatusCode, middleware, routing::get};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

pub mod domain;
pub mod fleet;
pub mod health;
pub mod ingestion;
pub mod metrics;
pub mod observability;
pub mod replay;
pub mod weather;

use fleet::{FleetStore, fleet_router, spawn_projection_worker};
use health::{CriticalWorkerRegistry, WorkerSnapshot};
use ingestion::IngestionSubscription;
use metrics::{ApiMetrics, observe_request};
use observability::correlate_request;
use replay::{ReplayHandle, ReplaySpeed, ReplayStatus};

pub const SERVICE_NAME: &str = "flight-tracker-api";

#[derive(Clone)]
pub struct ApiState {
    database: PgPool,
    replay: Option<ReplayHandle>,
    fleet: FleetStore,
    workers: CriticalWorkerRegistry,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    checks: HealthChecks,
    workers: Vec<WorkerSnapshot>,
}

#[derive(Debug, Serialize)]
struct HealthChecks {
    critical_workers: &'static str,
}

#[derive(Debug, Serialize)]
struct ReadinessResponse {
    status: &'static str,
    checks: ReadinessChecks,
}

#[derive(Debug, Serialize)]
struct ReadinessChecks {
    database: &'static str,
    postgis: &'static str,
    critical_workers: &'static str,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct SourceHealthView {
    id: Uuid,
    operator_id: Uuid,
    schema_version: i16,
    provider: String,
    feed: String,
    state: String,
    observed_at: DateTime<Utc>,
    last_attempt_at: DateTime<Utc>,
    last_success_at: Option<DateTime<Utc>>,
    newest_event_at: Option<DateTime<Utc>>,
    consecutive_failures: i32,
    delay_seconds: Option<i64>,
    stale_after_seconds: i64,
    last_error_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct SourceHealthResponse {
    data: Vec<SourceHealthView>,
}

#[derive(Debug, Serialize)]
struct ApiErrorResponse {
    error: ApiErrorBody,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    code: &'static str,
    message: &'static str,
}

pub fn build_router(database: PgPool) -> Router {
    build_router_with_runtime(database, None, CriticalWorkerRegistry::default())
}

pub fn build_router_with_replay(database: PgPool, replay: Option<ReplayHandle>) -> Router {
    build_router_with_runtime(database, replay, CriticalWorkerRegistry::default())
}

pub fn build_router_with_runtime(
    database: PgPool,
    replay: Option<ReplayHandle>,
    workers: CriticalWorkerRegistry,
) -> Router {
    build_router_with_runtime_and_ingestion(database, replay, workers, Vec::new())
}

pub fn build_router_with_runtime_and_ingestion(
    database: PgPool,
    replay: Option<ReplayHandle>,
    workers: CriticalWorkerRegistry,
    subscriptions: Vec<IngestionSubscription>,
) -> Router {
    let fleet = FleetStore::new(2_048);
    if let Some(handle) = replay.as_ref() {
        spawn_projection_worker(
            fleet.clone(),
            handle.subscribe(),
            workers.register("fleet_projection"),
        );
    }
    for subscription in subscriptions {
        spawn_projection_worker(
            fleet.clone(),
            subscription.receiver,
            workers.register(subscription.worker_name),
        );
    }
    build_router_with_services_and_health(database, replay, fleet, workers)
}

pub fn build_router_with_services(
    database: PgPool,
    replay: Option<ReplayHandle>,
    fleet: FleetStore,
) -> Router {
    build_router_with_services_and_health(
        database,
        replay,
        fleet,
        CriticalWorkerRegistry::default(),
    )
}

fn build_router_with_services_and_health(
    database: PgPool,
    replay: Option<ReplayHandle>,
    fleet: FleetStore,
    workers: CriticalWorkerRegistry,
) -> Router {
    let metrics = ApiMetrics::default();
    let fleet_routes = fleet_router(fleet.clone(), metrics.clone());
    let mut router = Router::new()
        .route("/health", get(health))
        .route("/readiness", get(readiness))
        .route("/api/source-health", get(source_health));

    if replay.is_some() {
        router = router
            .route("/api/dev/replay", get(replay_status))
            .route("/api/dev/replay/pause", axum::routing::post(replay_pause))
            .route("/api/dev/replay/resume", axum::routing::post(replay_resume))
            .route("/api/dev/replay/reset", axum::routing::post(replay_reset))
            .route("/api/dev/replay/speed", axum::routing::post(replay_speed))
            .route("/api/dev/replay/outage", axum::routing::post(replay_outage));
    }

    router
        .with_state(ApiState {
            database,
            replay,
            fleet,
            workers,
        })
        .merge(fleet_routes)
        .layer(middleware::from_fn_with_state(metrics, observe_request))
        .layer(middleware::from_fn(correlate_request))
}

#[derive(Debug, Deserialize)]
struct ReplaySpeedRequest {
    speed: ReplaySpeed,
}

#[derive(Debug, Deserialize)]
struct ReplayOutageRequest {
    active: bool,
}

async fn replay_status(State(state): State<ApiState>) -> Json<ReplayStatus> {
    Json(
        state
            .replay
            .expect("replay route requires handle")
            .status()
            .await,
    )
}

async fn replay_pause(State(state): State<ApiState>) -> Json<ReplayStatus> {
    Json(
        state
            .replay
            .expect("replay route requires handle")
            .pause()
            .await,
    )
}

async fn replay_resume(State(state): State<ApiState>) -> Json<ReplayStatus> {
    Json(
        state
            .replay
            .expect("replay route requires handle")
            .resume()
            .await,
    )
}

async fn replay_reset(State(state): State<ApiState>) -> Json<ReplayStatus> {
    let status = state
        .replay
        .as_ref()
        .expect("replay route requires handle")
        .reset()
        .await;
    state.fleet.clear_projection().await;
    Json(status)
}

async fn replay_speed(
    State(state): State<ApiState>,
    Json(request): Json<ReplaySpeedRequest>,
) -> Json<ReplayStatus> {
    Json(
        state
            .replay
            .expect("replay route requires handle")
            .set_speed(request.speed)
            .await,
    )
}

async fn replay_outage(
    State(state): State<ApiState>,
    Json(request): Json<ReplayOutageRequest>,
) -> Json<ReplayStatus> {
    Json(
        state
            .replay
            .expect("replay route requires handle")
            .set_feed_outage(request.active)
            .await,
    )
}

async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    let workers = state.workers.snapshot();
    let workers_ready = workers.iter().all(WorkerSnapshot::is_ready);
    Json(HealthResponse {
        status: if workers_ready { "ok" } else { "degraded" },
        service: SERVICE_NAME,
        version: env!("CARGO_PKG_VERSION"),
        checks: HealthChecks {
            critical_workers: if workers_ready { "ok" } else { "degraded" },
        },
        workers,
    })
}

async fn source_health(
    State(state): State<ApiState>,
) -> Result<Json<SourceHealthResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let rows = sqlx::query_as::<_, SourceHealthView>(
        r#"
        SELECT id, operator_id, schema_version, provider, feed, state, observed_at,
               last_attempt_at, last_success_at, newest_event_at, consecutive_failures,
               delay_seconds, stale_after_seconds, last_error_code
        FROM source_health
        ORDER BY provider, feed
        "#,
    )
    .fetch_all(&state.database)
    .await
    .map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiErrorResponse {
                error: ApiErrorBody {
                    code: "source_health_unavailable",
                    message: "Source health is temporarily unavailable",
                },
            }),
        )
    })?;
    Ok(Json(SourceHealthResponse { data: rows }))
}

async fn readiness(State(state): State<ApiState>) -> (StatusCode, Json<ReadinessResponse>) {
    let workers_ready = state.workers.is_ready();
    let postgis_ready = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'postgis')",
    )
    .fetch_one(&state.database)
    .await;

    match postgis_ready {
        Ok(true) => readiness_response(
            if workers_ready {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            "ok",
            "ok",
            workers_ready,
        ),
        Ok(false) => readiness_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "ok",
            "missing",
            workers_ready,
        ),
        Err(error) => {
            tracing::warn!(error = %error, "readiness database check failed");
            readiness_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "unknown",
                workers_ready,
            )
        }
    }
}

fn readiness_response(
    status_code: StatusCode,
    database: &'static str,
    postgis: &'static str,
    workers_ready: bool,
) -> (StatusCode, Json<ReadinessResponse>) {
    (
        status_code,
        Json(ReadinessResponse {
            status: if status_code == StatusCode::OK {
                "ready"
            } else {
                "not_ready"
            },
            checks: ReadinessChecks {
                database,
                postgis,
                critical_workers: if workers_ready { "ok" } else { "degraded" },
            },
        }),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use super::*;
    use crate::replay::ReplayScenario;

    fn unavailable_database() -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://flight_tracker:flight_tracker@127.0.0.1:1/flight_tracker")
            .expect("test database URL should be valid")
    }

    #[tokio::test]
    async fn health_reports_service_identity_without_database_access() {
        let response = build_router(unavailable_database())
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["service"], SERVICE_NAME);
    }

    #[tokio::test]
    async fn readiness_fails_closed_when_database_is_unavailable() {
        let response = build_router(unavailable_database())
            .oneshot(Request::get("/readiness").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"], "not_ready");
        assert_eq!(payload["checks"]["database"], "unavailable");
        assert_eq!(payload["checks"]["critical_workers"], "ok");
    }

    #[tokio::test]
    async fn health_reports_a_critical_worker_that_has_not_started() {
        let workers = CriticalWorkerRegistry::default();
        let _probe = workers.register("test_worker");
        let response = build_router_with_runtime(unavailable_database(), None, workers)
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"], "degraded");
        assert_eq!(payload["checks"]["critical_workers"], "degraded");
        assert_eq!(payload["workers"][0]["name"], "test_worker");
        assert_eq!(payload["workers"][0]["state"], "starting");
    }

    #[tokio::test]
    async fn replay_routes_do_not_exist_without_an_enabled_handle() {
        let response = build_router(unavailable_database())
            .oneshot(Request::get("/api/dev/replay").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn replay_routes_control_an_enabled_development_scenario() {
        let scenario = ReplayScenario::from_json(include_str!(
            "../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap();
        let app = build_router_with_replay(
            unavailable_database(),
            Some(ReplayHandle::new(scenario, 16)),
        );

        let status = app
            .clone()
            .oneshot(Request::get("/api/dev/replay").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(status.status(), StatusCode::OK);
        let body = status.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["phase"], "paused");
        assert_eq!(payload["total_events"], 12);

        let speed = app
            .clone()
            .oneshot(
                Request::post("/api/dev/replay/speed")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"speed":"4x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(speed.status(), StatusCode::OK);

        let resume = app
            .clone()
            .oneshot(
                Request::post("/api/dev/replay/resume")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resume.status(), StatusCode::OK);
        let body = resume.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["phase"], "running");
        assert_eq!(payload["speed"], "4x");

        let outage = app
            .oneshot(
                Request::post("/api/dev/replay/outage")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"active":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(outage.status(), StatusCode::OK);
        let body = outage.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["feed_outage"], true);
    }

    #[tokio::test]
    async fn replay_reset_clears_projected_fleet_state() {
        let scenario = ReplayScenario::from_json(include_str!(
            "../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap();
        let store = FleetStore::new(16);
        store
            .apply(&scenario.batch_for(&scenario.events[1]).unwrap())
            .await
            .unwrap();
        let app = build_router_with_services(
            unavailable_database(),
            Some(ReplayHandle::new(scenario, 16)),
            store,
        );

        let reset = app
            .clone()
            .oneshot(
                Request::post("/api/dev/replay/reset")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reset.status(), StatusCode::OK);

        let flights = app
            .oneshot(Request::get("/api/flights").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let payload = flights.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(payload["pagination"]["total_items"], 0);
    }

    #[tokio::test]
    async fn replay_runtime_projects_flights_through_public_api_routes() {
        let scenario = ReplayScenario::from_json(include_str!(
            "../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap();
        let handle = ReplayHandle::new(scenario, 64);
        let workers = CriticalWorkerRegistry::default();
        let runtime = crate::replay::spawn_replay_runtime(
            handle.clone(),
            Duration::from_millis(25),
            workers.register("test_replay"),
        );
        let app = build_router_with_runtime(unavailable_database(), Some(handle), workers);

        tokio::time::sleep(Duration::from_millis(30)).await;
        let health = app
            .clone()
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let payload = health.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["workers"].as_array().unwrap().len(), 2);

        app.clone()
            .oneshot(
                Request::post("/api/dev/replay/outage")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"active":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                Request::post("/api/dev/replay/speed")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"speed":"8x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        app.clone()
            .oneshot(
                Request::post("/api/dev/replay/resume")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let during_outage = app
            .clone()
            .oneshot(Request::get("/api/flights").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let payload = during_outage
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let payload: Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(payload["pagination"]["total_items"], 0);

        app.clone()
            .oneshot(
                Request::post("/api/dev/replay/outage")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"active":false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        let flights = app
            .clone()
            .oneshot(Request::get("/api/flights").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let payload = flights.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(payload["pagination"]["total_items"], 3);
        assert!(payload["data"].as_array().unwrap().iter().all(|flight| {
            flight["latest_position"].is_object() || flight["flight"]["callsign"] == "FT202"
        }));

        let reset = app
            .clone()
            .oneshot(
                Request::post("/api/dev/replay/reset")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reset.status(), StatusCode::OK);
        let flights = app
            .oneshot(Request::get("/api/flights").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let payload = flights.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(payload["pagination"]["total_items"], 0);
        runtime.abort();
    }
}
