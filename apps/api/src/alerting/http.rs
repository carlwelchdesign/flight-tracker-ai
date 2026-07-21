use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::OperatorId;

use super::{AlertActionRequest, AlertDetail, AlertQueueItem, AlertStore, AlertStoreError};

#[derive(Debug, Deserialize)]
struct QueueQuery {
    operator_id: OperatorId,
    #[serde(default)]
    include_terminal: bool,
}

#[derive(Debug, Deserialize)]
struct DetailQuery {
    operator_id: OperatorId,
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
    Query(query): Query<QueueQuery>,
) -> Result<Json<QueueResponse>, ApiError> {
    let data = store
        .list_queue(query.operator_id, query.include_terminal)
        .await
        .map_err(map_error)?;
    Ok(Json(QueueResponse { data }))
}

async fn alert_detail(
    State(store): State<AlertStore>,
    Path(alert_id): Path<Uuid>,
    Query(query): Query<DetailQuery>,
) -> Result<Json<AlertDetail>, ApiError> {
    store
        .detail(query.operator_id, alert_id)
        .await
        .map(Json)
        .map_err(map_error)
}

async fn apply_action(
    State(store): State<AlertStore>,
    Path(alert_id): Path<Uuid>,
    Json(request): Json<AlertActionRequest>,
) -> Result<Json<AlertDetail>, ApiError> {
    store
        .apply_action(alert_id, &request, Utc::now())
        .await
        .map(Json)
        .map_err(map_error)
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
