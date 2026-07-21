use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{AuthContext, Permission, require},
    domain::AlertActionKind,
};

use super::{AlertActionRequest, AlertDetail, AlertQueueItem, AlertStore, AlertStoreError};

#[derive(Debug, Deserialize)]
struct QueueQuery {
    #[serde(default)]
    include_terminal: bool,
}

#[derive(Debug, Deserialize)]
struct ApplyActionBody {
    action: AlertActionKind,
    idempotency_key: String,
    comment: Option<String>,
}

#[derive(Debug, Serialize)]
struct QueueResponse {
    data: Vec<AlertQueueItem>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

type ApiError = (StatusCode, Json<ErrorResponse>);

pub fn alert_router(store: AlertStore) -> Router {
    Router::new()
        .route("/api/alerts", get(list_alerts))
        .route("/api/alerts/{alert_id}", get(alert_detail))
        .route("/api/alerts/{alert_id}/actions", post(apply_action))
        .with_state(store)
}

async fn list_alerts(
    State(store): State<AlertStore>,
    Extension(context): Extension<AuthContext>,
    Query(query): Query<QueueQuery>,
) -> Result<Json<QueueResponse>, axum::response::Response> {
    require(&context, Permission::ReadOperations)
        .map_err(axum::response::IntoResponse::into_response)?;
    let data = store
        .list_queue(context.operator_id, query.include_terminal)
        .await
        .map_err(|error| axum::response::IntoResponse::into_response(map_error(error)))?;
    Ok(Json(QueueResponse { data }))
}

async fn alert_detail(
    State(store): State<AlertStore>,
    Extension(context): Extension<AuthContext>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<AlertDetail>, axum::response::Response> {
    require(&context, Permission::ReadOperations)
        .map_err(axum::response::IntoResponse::into_response)?;
    store
        .detail(context.operator_id, alert_id)
        .await
        .map(Json)
        .map_err(|error| axum::response::IntoResponse::into_response(map_error(error)))
}

async fn apply_action(
    State(store): State<AlertStore>,
    Extension(context): Extension<AuthContext>,
    Path(alert_id): Path<Uuid>,
    Json(body): Json<ApplyActionBody>,
) -> Result<Json<AlertDetail>, axum::response::Response> {
    require(&context, Permission::ManageAlerts)
        .map_err(axum::response::IntoResponse::into_response)?;
    let request = AlertActionRequest {
        operator_id: context.operator_id,
        action: body.action,
        actor_id: context.subject,
        idempotency_key: body.idempotency_key,
        comment: body.comment,
    };
    store
        .apply_action(alert_id, &request, Utc::now())
        .await
        .map(Json)
        .map_err(|error| axum::response::IntoResponse::into_response(map_error(error)))
}

fn map_error(error: AlertStoreError) -> ApiError {
    let (status, code) = match error {
        AlertStoreError::NotFound => (StatusCode::NOT_FOUND, "alert_not_found"),
        AlertStoreError::InvalidActionIdentity
        | AlertStoreError::IdempotencyConflict
        | AlertStoreError::Lifecycle(_) => (StatusCode::CONFLICT, "invalid_alert_action"),
        AlertStoreError::Database(_) | AlertStoreError::InvalidStoredLifecycle => {
            (StatusCode::SERVICE_UNAVAILABLE, "alert_service_unavailable")
        }
    };
    (
        status,
        Json(ErrorResponse {
            error: ErrorBody {
                code,
                message: error.to_string(),
            },
        }),
    )
}
