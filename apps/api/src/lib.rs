use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

pub mod domain;
pub mod fleet;
pub mod ingestion;
pub mod metrics;
pub mod replay;

use fleet::{FleetStore, fleet_router, spawn_projection_worker};
use metrics::ApiMetrics;
use replay::{ReplayHandle, ReplaySpeed, ReplayStatus};

pub const SERVICE_NAME: &str = "flight-tracker-api";

#[derive(Clone)]
pub struct ApiState {
    database: PgPool,
    replay: Option<ReplayHandle>,
    fleet: FleetStore,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
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
}

pub fn build_router(database: PgPool) -> Router {
    build_router_with_services(database, None, FleetStore::new(2_048))
}

pub fn build_router_with_replay(database: PgPool, replay: Option<ReplayHandle>) -> Router {
    let fleet = FleetStore::new(2_048);
    if let Some(handle) = replay.as_ref() {
        spawn_projection_worker(fleet.clone(), handle.subscribe());
    }
    build_router_with_services(database, replay, fleet)
}

pub fn build_router_with_services(
    database: PgPool,
    replay: Option<ReplayHandle>,
    fleet: FleetStore,
) -> Router {
    let metrics = ApiMetrics::default();
    let fleet_routes = fleet_router(fleet.clone(), metrics);
    let mut router = Router::new()
        .route("/health", get(health))
        .route("/readiness", get(readiness));

    if replay.is_some() {
        router = router
            .route("/api/dev/replay", get(replay_status))
            .route("/api/dev/replay/pause", axum::routing::post(replay_pause))
            .route("/api/dev/replay/resume", axum::routing::post(replay_resume))
            .route("/api/dev/replay/reset", axum::routing::post(replay_reset))
            .route("/api/dev/replay/speed", axum::routing::post(replay_speed));
    }

    router
        .with_state(ApiState {
            database,
            replay,
            fleet,
        })
        .merge(fleet_routes)
        .layer(TraceLayer::new_for_http())
}

#[derive(Debug, Deserialize)]
struct ReplaySpeedRequest {
    speed: ReplaySpeed,
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

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: SERVICE_NAME,
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn readiness(State(state): State<ApiState>) -> (StatusCode, Json<ReadinessResponse>) {
    let postgis_ready = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'postgis')",
    )
    .fetch_one(&state.database)
    .await;

    match postgis_ready {
        Ok(true) => (
            StatusCode::OK,
            Json(ReadinessResponse {
                status: "ready",
                checks: ReadinessChecks {
                    database: "ok",
                    postgis: "ok",
                },
            }),
        ),
        Ok(false) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadinessResponse {
                status: "not_ready",
                checks: ReadinessChecks {
                    database: "ok",
                    postgis: "missing",
                },
            }),
        ),
        Err(error) => {
            tracing::warn!(error = %error, "readiness database check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ReadinessResponse {
                    status: "not_ready",
                    checks: ReadinessChecks {
                        database: "unavailable",
                        postgis: "unknown",
                    },
                }),
            )
        }
    }
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
        let runtime =
            crate::replay::spawn_replay_runtime(handle.clone(), Duration::from_millis(25));
        let app = build_router_with_replay(unavailable_database(), Some(handle));

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
