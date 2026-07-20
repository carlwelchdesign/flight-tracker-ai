use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

pub const SERVICE_NAME: &str = "flight-tracker-api";

#[derive(Clone)]
pub struct ApiState {
    database: PgPool,
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
    Router::new()
        .route("/health", get(health))
        .route("/readiness", get(readiness))
        .with_state(ApiState { database })
        .layer(TraceLayer::new_for_http())
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
}
