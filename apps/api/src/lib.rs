use axum::{
    Json, Router,
    extract::{Extension, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{FlightId, OperatorId};

pub mod alerting;
pub mod auth;
pub mod domain;
pub mod fleet;
pub mod health;
pub mod ingestion;
pub mod live_positions;
pub mod metrics;
pub mod observability;
pub mod replay;
pub mod retention;
pub mod weather;

use alerting::{AlertStore, alert_router, spawn_alert_worker};
use auth::{
    AuditStore, AuthContext, AuthFailure, AuthService, Permission, audit_router, auth_router,
    authenticate_request, require,
};
use fleet::{FleetStore, FlightView, fleet_router, spawn_projection_worker};
use health::{CriticalWorkerRegistry, WorkerSnapshot};
use ingestion::IngestionSubscription;
use live_positions::{LivePositionStatus, LivePositionStatusStore};
use metrics::{ApiMetrics, observe_request};
use observability::correlate_request;
use replay::{ReplayHandle, ReplaySpeed, ReplayStatus};
use retention::{RetentionStore, retention_router};
use weather::{public_weather_router, weather_router};

pub const SERVICE_NAME: &str = "flight-tracker-api";

#[derive(Clone, Copy, Default)]
pub struct PublicPortfolioOperators {
    pub live_positions: Option<OperatorId>,
    pub weather: Option<OperatorId>,
}

#[derive(Clone)]
pub struct ApiState {
    database: PgPool,
    replay: Option<ReplayHandle>,
    fleet: FleetStore,
    workers: CriticalWorkerRegistry,
    live_positions: LivePositionStatusStore,
    public_live_operator: Option<OperatorId>,
}

const PUBLIC_LIVE_PROVIDER: &str = "adsb.lol";
const PUBLIC_LIVE_LIMIT: usize = 500;

#[derive(Debug, Serialize)]
struct PublicLivePositionSnapshot {
    status: LivePositionStatus,
    data: Vec<PublicAircraftPosition>,
}

#[derive(Debug, Serialize)]
struct PublicAircraftPosition {
    id: FlightId,
    callsign: Option<String>,
    aircraft_registration: Option<String>,
    longitude_degrees: f64,
    latitude_degrees: f64,
    altitude: Option<domain::Altitude>,
    heading_true_degrees: Option<f64>,
    ground_speed: Option<domain::Speed>,
    quality: domain::SourceQuality,
    observed_at: DateTime<Utc>,
    received_at: DateTime<Utc>,
    provider: String,
}

impl From<FlightView> for PublicAircraftPosition {
    fn from(view: FlightView) -> Self {
        let position = view
            .latest_position
            .expect("provider position query only returns positioned aircraft");
        let [longitude_degrees, latitude_degrees] = position.point.as_geojson_position();
        Self {
            id: view.flight.id,
            callsign: view.flight.callsign,
            aircraft_registration: view.flight.aircraft_registration,
            longitude_degrees,
            latitude_degrees,
            altitude: position.altitude,
            heading_true_degrees: position.heading_true_degrees.map(Into::into),
            ground_speed: position.ground_speed,
            quality: position.quality,
            observed_at: position.times.event_time,
            received_at: position.times.received_at,
            provider: position.source.provider,
        }
    }
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
struct ProbeResponse {
    status: &'static str,
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

#[derive(Debug, Clone, Copy)]
struct ReadinessSnapshot {
    status_code: StatusCode,
    database: &'static str,
    postgis: &'static str,
    workers_ready: bool,
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

pub fn build_router(database: PgPool, auth: AuthService) -> Router {
    build_router_with_runtime(database, None, CriticalWorkerRegistry::default(), auth)
}

pub fn build_router_with_replay(
    database: PgPool,
    replay: Option<ReplayHandle>,
    auth: AuthService,
) -> Router {
    build_router_with_runtime(database, replay, CriticalWorkerRegistry::default(), auth)
}

pub fn build_router_with_runtime(
    database: PgPool,
    replay: Option<ReplayHandle>,
    workers: CriticalWorkerRegistry,
    auth: AuthService,
) -> Router {
    build_router_with_runtime_and_ingestion(database, replay, workers, Vec::new(), auth)
}

pub fn build_router_with_runtime_and_ingestion(
    database: PgPool,
    replay: Option<ReplayHandle>,
    workers: CriticalWorkerRegistry,
    subscriptions: Vec<IngestionSubscription>,
    auth: AuthService,
) -> Router {
    build_router_with_runtime_and_live_positions(
        database,
        replay,
        workers,
        subscriptions,
        LivePositionStatusStore::default(),
        auth,
    )
}

pub fn build_router_with_runtime_and_live_positions(
    database: PgPool,
    replay: Option<ReplayHandle>,
    workers: CriticalWorkerRegistry,
    subscriptions: Vec<IngestionSubscription>,
    live_positions: LivePositionStatusStore,
    auth: AuthService,
) -> Router {
    build_router_with_runtime_and_public_live_positions(
        database,
        replay,
        workers,
        subscriptions,
        live_positions,
        PublicPortfolioOperators::default(),
        auth,
    )
}

pub fn build_router_with_runtime_and_public_live_positions(
    database: PgPool,
    replay: Option<ReplayHandle>,
    workers: CriticalWorkerRegistry,
    subscriptions: Vec<IngestionSubscription>,
    live_positions: LivePositionStatusStore,
    public_operators: PublicPortfolioOperators,
    auth: AuthService,
) -> Router {
    let fleet = FleetStore::new(2_048);
    if let Some(handle) = replay.as_ref() {
        spawn_alert_worker(
            database.clone(),
            handle.subscribe(),
            workers.register("alert_projection"),
        );
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
    build_router_with_services_and_health(
        database,
        replay,
        fleet,
        workers,
        live_positions,
        public_operators,
        auth,
    )
}

pub fn build_router_with_services(
    database: PgPool,
    replay: Option<ReplayHandle>,
    fleet: FleetStore,
    auth: AuthService,
) -> Router {
    build_router_with_services_and_health(
        database,
        replay,
        fleet,
        CriticalWorkerRegistry::default(),
        LivePositionStatusStore::default(),
        PublicPortfolioOperators::default(),
        auth,
    )
}

fn build_router_with_services_and_health(
    database: PgPool,
    replay: Option<ReplayHandle>,
    fleet: FleetStore,
    workers: CriticalWorkerRegistry,
    live_positions: LivePositionStatusStore,
    public_operators: PublicPortfolioOperators,
    auth: AuthService,
) -> Router {
    let audit_store = AuditStore::new(database.clone());
    let retention_store = RetentionStore::new(database.clone());
    let metrics = ApiMetrics::default();
    let fleet_routes = fleet_router(fleet.clone(), metrics.clone());
    let weather_routes = weather_router(database.clone());
    let public_weather_routes = public_weather_router(database.clone(), public_operators.weather);
    let alert_routes = alert_router(AlertStore::new(database.clone()));
    let public = Router::new()
        .route("/health", get(health))
        .route("/readiness", get(readiness))
        .route("/api/public/live-positions", get(public_live_positions))
        .with_state(ApiState {
            database: database.clone(),
            replay: replay.clone(),
            fleet: fleet.clone(),
            workers: workers.clone(),
            live_positions: live_positions.clone(),
            public_live_operator: public_operators.live_positions,
        });
    let mut protected = Router::new()
        .route("/api/source-health", get(source_health))
        .route("/api/live-positions/status", get(live_position_status))
        .route("/api/system/health", get(system_health))
        .route("/api/system/readiness", get(system_readiness));

    if replay.is_some() {
        protected = protected
            .route("/api/dev/replay", get(replay_status))
            .route("/api/dev/replay/pause", axum::routing::post(replay_pause))
            .route("/api/dev/replay/resume", axum::routing::post(replay_resume))
            .route("/api/dev/replay/reset", axum::routing::post(replay_reset))
            .route("/api/dev/replay/speed", axum::routing::post(replay_speed))
            .route("/api/dev/replay/outage", axum::routing::post(replay_outage));
    }

    protected
        .with_state(ApiState {
            database,
            replay,
            fleet,
            workers,
            live_positions,
            public_live_operator: public_operators.live_positions,
        })
        .merge(fleet_routes)
        .merge(weather_routes)
        .merge(alert_routes)
        .merge(auth_router(auth.clone()))
        .merge(audit_router(audit_store))
        .merge(retention_router(retention_store))
        .layer(middleware::from_fn_with_state(auth, authenticate_request))
        .merge(public)
        .merge(public_weather_routes)
        .layer(middleware::from_fn_with_state(metrics, observe_request))
        .layer(middleware::from_fn(correlate_request))
        .layer(middleware::from_fn(secure_transport_headers))
}

async fn public_live_positions(State(state): State<ApiState>) -> impl IntoResponse {
    let now = Utc::now();
    let Some(operator_id) = state.public_live_operator else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CACHE_CONTROL, "no-store")],
            Json(PublicLivePositionSnapshot {
                status: LivePositionStatusStore::default().snapshot(OperatorId::new(), now),
                data: Vec::new(),
            }),
        );
    };
    let status = state.live_positions.snapshot(operator_id, now);
    let visible_window_seconds = status.stale_after_seconds.saturating_mul(4).max(120);
    let observed_after = now - chrono::Duration::seconds(visible_window_seconds as i64);
    let data = state
        .fleet
        .list_provider_positions(
            operator_id,
            PUBLIC_LIVE_PROVIDER,
            observed_after,
            PUBLIC_LIVE_LIMIT,
        )
        .await
        .into_iter()
        .map(PublicAircraftPosition::from)
        .collect();
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "no-store")],
        Json(PublicLivePositionSnapshot { status, data }),
    )
}

async fn secure_transport_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    response
}

#[derive(Debug, Deserialize)]
struct ReplaySpeedRequest {
    speed: ReplaySpeed,
}

#[derive(Debug, Deserialize)]
struct ReplayOutageRequest {
    active: bool,
}

async fn replay_status(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<ReplayStatus>, Response> {
    authorize_replay(&state, &context).await?;
    Ok(Json(
        state
            .replay
            .expect("replay route requires handle")
            .status()
            .await,
    ))
}

async fn replay_pause(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<ReplayStatus>, Response> {
    authorize_replay(&state, &context).await?;
    Ok(Json(
        state
            .replay
            .expect("replay route requires handle")
            .pause()
            .await,
    ))
}

async fn replay_resume(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<ReplayStatus>, Response> {
    authorize_replay(&state, &context).await?;
    Ok(Json(
        state
            .replay
            .expect("replay route requires handle")
            .resume()
            .await,
    ))
}

async fn replay_reset(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<ReplayStatus>, Response> {
    authorize_replay(&state, &context).await?;
    let status = state
        .replay
        .as_ref()
        .expect("replay route requires handle")
        .reset()
        .await;
    state.fleet.clear_projection().await;
    Ok(Json(status))
}

async fn replay_speed(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
    Json(request): Json<ReplaySpeedRequest>,
) -> Result<Json<ReplayStatus>, Response> {
    authorize_replay(&state, &context).await?;
    Ok(Json(
        state
            .replay
            .expect("replay route requires handle")
            .set_speed(request.speed)
            .await,
    ))
}

async fn replay_outage(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
    Json(request): Json<ReplayOutageRequest>,
) -> Result<Json<ReplayStatus>, Response> {
    authorize_replay(&state, &context).await?;
    Ok(Json(
        state
            .replay
            .expect("replay route requires handle")
            .set_feed_outage(request.active)
            .await,
    ))
}

async fn authorize_replay(state: &ApiState, context: &AuthContext) -> Result<(), Response> {
    require(context, Permission::ControlReplay).map_err(IntoResponse::into_response)?;
    let handle = state.replay.as_ref().expect("replay route requires handle");
    if handle.operator_id().await != context.operator_id {
        return Err(AuthFailure::Forbidden.into_response());
    }
    Ok(())
}

async fn health() -> Json<ProbeResponse> {
    Json(ProbeResponse { status: "ok" })
}

async fn system_health(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<HealthResponse>, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    let workers = state.workers.snapshot();
    let workers_ready = workers.iter().all(WorkerSnapshot::is_ready);
    Ok(Json(HealthResponse {
        status: if workers_ready { "ok" } else { "degraded" },
        service: SERVICE_NAME,
        version: env!("CARGO_PKG_VERSION"),
        checks: HealthChecks {
            critical_workers: if workers_ready { "ok" } else { "degraded" },
        },
        workers,
    }))
}

async fn source_health(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<SourceHealthResponse>, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    let rows = sqlx::query_as::<_, SourceHealthView>(
        r#"
        SELECT id, operator_id, schema_version, provider, feed, state, observed_at,
               last_attempt_at, last_success_at, newest_event_at, consecutive_failures,
               delay_seconds, stale_after_seconds, last_error_code
        FROM source_health
        WHERE operator_id = $1
        ORDER BY provider, feed
        "#,
    )
    .bind(context.operator_id.as_uuid())
    .fetch_all(&state.database)
    .await
    .map_err(|_| {
        IntoResponse::into_response((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiErrorResponse {
                error: ApiErrorBody {
                    code: "source_health_unavailable",
                    message: "Source health is temporarily unavailable",
                },
            }),
        ))
    })?;
    Ok(Json(SourceHealthResponse { data: rows }))
}

async fn live_position_status(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
) -> Result<impl IntoResponse, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    let status: LivePositionStatus = state
        .live_positions
        .snapshot(context.operator_id, Utc::now());
    Ok((
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(status),
    ))
}

async fn readiness(State(state): State<ApiState>) -> (StatusCode, Json<ProbeResponse>) {
    let snapshot = readiness_snapshot(&state).await;
    (
        snapshot.status_code,
        Json(ProbeResponse {
            status: if snapshot.status_code == StatusCode::OK {
                "ready"
            } else {
                "not_ready"
            },
        }),
    )
}

async fn system_readiness(
    State(state): State<ApiState>,
    Extension(context): Extension<AuthContext>,
) -> Result<(StatusCode, Json<ReadinessResponse>), Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    Ok(readiness_response(readiness_snapshot(&state).await))
}

async fn readiness_snapshot(state: &ApiState) -> ReadinessSnapshot {
    let workers_ready = state.workers.is_ready();
    let postgis_ready = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'postgis')",
    )
    .fetch_one(&state.database)
    .await;

    match postgis_ready {
        Ok(true) => ReadinessSnapshot {
            status_code: if workers_ready {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            database: "ok",
            postgis: "ok",
            workers_ready,
        },
        Ok(false) => ReadinessSnapshot {
            status_code: StatusCode::SERVICE_UNAVAILABLE,
            database: "ok",
            postgis: "missing",
            workers_ready,
        },
        Err(error) => {
            tracing::warn!(error = %error, "readiness database check failed");
            ReadinessSnapshot {
                status_code: StatusCode::SERVICE_UNAVAILABLE,
                database: "unavailable",
                postgis: "unknown",
                workers_ready,
            }
        }
    }
}

fn readiness_response(snapshot: ReadinessSnapshot) -> (StatusCode, Json<ReadinessResponse>) {
    (
        snapshot.status_code,
        Json(ReadinessResponse {
            status: if snapshot.status_code == StatusCode::OK {
                "ready"
            } else {
                "not_ready"
            },
            checks: ReadinessChecks {
                database: snapshot.database,
                postgis: snapshot.postgis,
                critical_workers: if snapshot.workers_ready {
                    "ok"
                } else {
                    "degraded"
                },
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
    use crate::{
        auth::{AssertionConfig, AssertionKey, AuthStore, InternalAssertionVerifier},
        domain::CanonicalEvent,
        live_positions::LivePositionRegion,
        replay::ReplayScenario,
    };

    fn unavailable_database() -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://flight_tracker:flight_tracker@127.0.0.1:1/flight_tracker")
            .expect("test database URL should be valid")
    }

    fn test_auth(database: &PgPool) -> AuthService {
        AuthService::new(
            InternalAssertionVerifier::new(AssertionConfig {
                active_key: AssertionKey {
                    id: "test-primary".into(),
                    secret: "test-only-internal-assertion-secret-32-bytes".into(),
                },
                previous_key: None,
                issuer: "test-web".into(),
                audience: "test-api".into(),
                leeway_seconds: 0,
            })
            .unwrap(),
            AuthStore::new(database.clone()),
        )
    }

    #[tokio::test]
    async fn public_health_is_minimal_and_does_not_access_the_database() {
        let database = unavailable_database();
        let response = build_router(database.clone(), test_auth(&database))
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::STRICT_TRANSPORT_SECURITY),
            Some(&HeaderValue::from_static(
                "max-age=31536000; includeSubDomains"
            ))
        );
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload, serde_json::json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn readiness_fails_closed_when_database_is_unavailable() {
        let database = unavailable_database();
        let response = build_router(database.clone(), test_auth(&database))
            .oneshot(Request::get("/readiness").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload, serde_json::json!({ "status": "not_ready" }));
    }

    #[tokio::test]
    async fn public_health_does_not_expose_critical_worker_details() {
        let workers = CriticalWorkerRegistry::default();
        let _probe = workers.register("test_worker");
        let database = unavailable_database();
        let response =
            build_router_with_runtime(database.clone(), None, workers, test_auth(&database))
                .oneshot(Request::get("/health").body(Body::empty()).unwrap())
                .await
                .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload, serde_json::json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn public_live_positions_fail_closed_and_never_cache_when_disabled() {
        let database = unavailable_database();
        let response = build_router(database.clone(), test_auth(&database))
            .oneshot(
                Request::get("/api/public/live-positions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
    }

    #[tokio::test]
    async fn public_weather_fails_closed_and_never_caches_when_disabled() {
        let database = unavailable_database();
        let response = build_router(database.clone(), test_auth(&database))
            .oneshot(
                Request::get("/api/public/weather")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        let payload: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(payload["state"], "unavailable");
        assert_eq!(payload["hazards"], serde_json::json!([]));
        assert_eq!(payload["observations"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn public_live_positions_are_operator_bound_and_sanitized() {
        let scenario = ReplayScenario::from_json(include_str!(
            "../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap();
        let fleet = FleetStore::new(32);
        for event in [&scenario.events[1], &scenario.events[5]] {
            let mut batch = scenario.batch_for(event).unwrap();
            batch.envelope.provider = PUBLIC_LIVE_PROVIDER.into();
            batch.envelope.feed = "point".into();
            match &mut batch.events[0] {
                CanonicalEvent::Flight(value) => {
                    value.source.provider = PUBLIC_LIVE_PROVIDER.into();
                    value.source.feed = "point".into();
                    value.times.event_time = Utc::now();
                }
                CanonicalEvent::AircraftPosition(value) => {
                    value.source.provider = PUBLIC_LIVE_PROVIDER.into();
                    value.source.feed = "point".into();
                    value.times.event_time = Utc::now();
                }
                _ => panic!("fixture event must project a flight position"),
            }
            fleet.apply(&batch).await.unwrap();
        }
        let live_positions = LivePositionStatusStore::default();
        live_positions.register(
            scenario.operator_id,
            LivePositionRegion {
                latitude_degrees: 37.62,
                longitude_degrees: -122.38,
                radius_nautical_miles: 25,
            },
            Duration::from_secs(30),
            Utc::now(),
        );
        let database = unavailable_database();
        let app = build_router_with_services_and_health(
            database.clone(),
            None,
            fleet,
            CriticalWorkerRegistry::default(),
            live_positions,
            PublicPortfolioOperators {
                live_positions: Some(scenario.operator_id),
                weather: None,
            },
            test_auth(&database),
        );
        let response = app
            .oneshot(
                Request::get("/api/public/live-positions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        let payload: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(payload["data"].as_array().unwrap().len(), 1);
        assert_eq!(payload["data"][0]["provider"], PUBLIC_LIVE_PROVIDER);
        assert!(payload["data"][0].get("operator_id").is_none());
        assert!(payload["data"][0].get("source").is_none());
        assert!(payload["data"][0].get("origin_airport_code").is_none());
        assert!(payload["data"][0].get("destination_airport_code").is_none());
    }

    #[tokio::test]
    async fn replay_routes_do_not_exist_without_an_enabled_handle() {
        let database = unavailable_database();
        let response = build_router(database.clone(), test_auth(&database))
            .oneshot(Request::get("/api/dev/replay").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn operational_and_replay_routes_require_a_bearer_assertion() {
        let scenario = ReplayScenario::from_json(include_str!(
            "../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap();
        let database = unavailable_database();
        let app = build_router_with_replay(
            database.clone(),
            Some(ReplayHandle::new(scenario, 16)),
            test_auth(&database),
        );

        for request in [
            Request::get("/api/dev/replay").body(Body::empty()).unwrap(),
            Request::post("/api/dev/replay/resume")
                .body(Body::empty())
                .unwrap(),
            Request::get("/api/flights").body(Body::empty()).unwrap(),
            Request::get("/api/events/stream")
                .body(Body::empty())
                .unwrap(),
            Request::get("/api/system/health")
                .body(Body::empty())
                .unwrap(),
            Request::get("/api/system/readiness")
                .body(Body::empty())
                .unwrap(),
            Request::get("/api/live-positions/status")
                .body(Body::empty())
                .unwrap(),
            Request::get("/metrics").body(Body::empty()).unwrap(),
        ] {
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }
}
