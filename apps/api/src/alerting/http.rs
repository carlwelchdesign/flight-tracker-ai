use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{AuthContext, Permission, require},
    domain::AlertActionKind,
};

use super::{
    AlertActionRequest, AlertAssignee, AlertDetail, AlertQueueFilter, AlertQueueItem, AlertStore,
    AlertStoreError, AssignmentFilter, DismissalReason,
};

#[derive(Debug, Deserialize)]
struct QueueQuery {
    #[serde(default)]
    include_terminal: bool,
    severity: Option<String>,
    status: Option<String>,
    flight: Option<String>,
    event_from: Option<DateTime<Utc>>,
    event_to: Option<DateTime<Utc>>,
    assigned_to: Option<String>,
    #[serde(default = "default_queue_limit")]
    limit: i64,
}

#[derive(Debug, Deserialize)]
struct ApplyActionBody {
    action: AlertActionKind,
    idempotency_key: String,
    expected_workflow_version: i32,
    comment: Option<String>,
    assigned_identity_id: Option<Uuid>,
    dismissal_reason: Option<DismissalReason>,
}

#[derive(Debug, Serialize)]
struct QueueResponse {
    data: Vec<AlertQueueItem>,
}

#[derive(Debug, Serialize)]
struct AssigneeResponse {
    data: Vec<AlertAssignee>,
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
        .route("/api/alerts/assignees", get(list_assignees))
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
    let filter = query
        .into_filter()
        .map_err(axum::response::IntoResponse::into_response)?;
    let data = store
        .list_queue(context.operator_id, &filter)
        .await
        .map_err(|error| axum::response::IntoResponse::into_response(map_error(error)))?;
    Ok(Json(QueueResponse { data }))
}

async fn list_assignees(
    State(store): State<AlertStore>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<AssigneeResponse>, axum::response::Response> {
    require(&context, Permission::ReadOperations)
        .map_err(axum::response::IntoResponse::into_response)?;
    let data = store
        .list_assignees(context.operator_id)
        .await
        .map_err(|error| axum::response::IntoResponse::into_response(map_error(error)))?;
    Ok(Json(AssigneeResponse { data }))
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
        expected_workflow_version: body.expected_workflow_version,
        comment: body.comment,
        assigned_identity_id: body.assigned_identity_id,
        dismissal_reason: body.dismissal_reason,
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
        AlertStoreError::ConcurrentModification => (StatusCode::CONFLICT, "alert_conflict"),
        AlertStoreError::InvalidAssignee => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_assignee"),
        AlertStoreError::InvalidDismissalReason => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid_dismissal_reason")
        }
        AlertStoreError::InvalidComment => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_comment"),
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

impl QueueQuery {
    fn into_filter(self) -> Result<AlertQueueFilter, ApiError> {
        if self.severity.as_deref().is_some_and(|value| {
            !matches!(value, "information" | "advisory" | "warning" | "critical")
        }) {
            return Err(invalid_query("severity is not supported"));
        }
        if self.status.as_deref().is_some_and(|value| {
            !matches!(value, "open" | "acknowledged" | "dismissed" | "resolved")
        }) {
            return Err(invalid_query("status is not supported"));
        }
        if self
            .event_from
            .zip(self.event_to)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(invalid_query("event_from must not be later than event_to"));
        }
        let assignment = match self.assigned_to.as_deref() {
            None => None,
            Some("unassigned") => Some(AssignmentFilter::Unassigned),
            Some(value) => Some(AssignmentFilter::Identity(Uuid::parse_str(value).map_err(
                |_| invalid_query("assigned_to must be an identity UUID or unassigned"),
            )?)),
        };
        Ok(AlertQueueFilter {
            include_terminal: self.include_terminal,
            severity: self.severity,
            lifecycle: self.status,
            flight: self.flight.filter(|value| !value.trim().is_empty()),
            event_from: self.event_from,
            event_to: self.event_to,
            assignment,
            limit: self.limit,
        })
    }
}

const fn default_queue_limit() -> i64 {
    200
}

fn invalid_query(message: &str) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: ErrorBody {
                code: "invalid_alert_filter",
                message: message.to_owned(),
            },
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> QueueQuery {
        QueueQuery {
            include_terminal: false,
            severity: None,
            status: None,
            flight: None,
            event_from: None,
            event_to: None,
            assigned_to: None,
            limit: default_queue_limit(),
        }
    }

    #[test]
    fn queue_filters_validate_operational_values() {
        let mut invalid = query();
        invalid.severity = Some("emergency".into());
        assert!(invalid.into_filter().is_err());

        let mut assignment = query();
        assignment.assigned_to = Some("unassigned".into());
        assert_eq!(
            assignment.into_filter().unwrap().assignment,
            Some(AssignmentFilter::Unassigned)
        );
    }

    #[test]
    fn queue_time_range_must_be_ordered() {
        let mut invalid = query();
        invalid.event_from = Some("2026-07-21T02:00:00Z".parse().unwrap());
        invalid.event_to = Some("2026-07-21T01:00:00Z".parse().unwrap());
        assert!(invalid.into_filter().is_err());
    }
}
