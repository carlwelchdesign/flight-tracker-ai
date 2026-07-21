use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Extension, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::OperatorId;

use super::{AuthContext, Permission, require};

const DEFAULT_REVIEW_HOURS: i64 = 24;
const MAX_EXPORT_DAYS: i64 = 31;
const MAX_EXPORT_EVENTS: i64 = 10_000;
const MAX_MONITOR_EVENTS: i64 = 10_000;
const BURST_WINDOW_MINUTES: i64 = 15;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditRisk {
    #[default]
    Routine,
    Sensitive,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct AuditEventView {
    pub id: Uuid,
    pub source: String,
    pub actor_id: String,
    pub action: String,
    pub target_type: String,
    pub target_reference: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub details: Value,
    #[sqlx(skip)]
    pub risk: AuditRisk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditSignalView {
    pub code: &'static str,
    pub severity: &'static str,
    pub actor_id: String,
    pub occurred_at: DateTime<Utc>,
    pub event_id: Option<Uuid>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct ReviewQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ExportQuery {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct MonitorQuery {
    since: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct AuditEventList {
    data: Vec<AuditEventView>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AuditSignalList {
    data: Vec<AuditSignalView>,
    since: DateTime<Utc>,
    through: DateTime<Utc>,
}

#[derive(Debug, Error)]
enum AuditError {
    #[error("the audit time range is invalid or exceeds the allowed window")]
    InvalidRange,
    #[error("the audit result exceeds the safe export or monitoring limit")]
    ScopeTooLarge,
    #[error("audit review is temporarily unavailable")]
    Unavailable,
    #[error("the current session is not authorized to review audit records")]
    Forbidden,
}

impl IntoResponse for AuditError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::InvalidRange => (StatusCode::BAD_REQUEST, "invalid_audit_range"),
            Self::ScopeTooLarge => (StatusCode::UNPROCESSABLE_ENTITY, "audit_scope_too_large"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "audit_unavailable"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "authorization_denied"),
        };
        (
            status,
            Json(json!({ "error": { "code": code, "message": self.to_string() } })),
        )
            .into_response()
    }
}

#[derive(Clone)]
pub struct AuditStore {
    database: PgPool,
}

impl AuditStore {
    pub fn new(database: PgPool) -> Self {
        Self { database }
    }

    pub async fn events(
        &self,
        operator_id: OperatorId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<AuditEventView>, sqlx::Error> {
        let mut events = sqlx::query_as::<_, AuditEventView>(
            r#"
            WITH audit_events AS (
                SELECT event.id, 'authorization'::text AS source,
                       event.actor_identity_id::text AS actor_id, event.action,
                       event.target_type,
                       CASE WHEN event.target_type = 'auth_session' THEN NULL
                            ELSE event.target_id END AS target_reference,
                       event.occurred_at,
                       CASE event.action
                         WHEN 'membership.updated' THEN jsonb_strip_nulls(jsonb_build_object(
                           'role', event.metadata->>'role', 'status', event.metadata->>'status'))
                         WHEN 'session.revoked' THEN jsonb_strip_nulls(jsonb_build_object(
                           'provider', event.metadata->>'provider',
                           'identity_id', event.metadata->>'identity_id'))
                         ELSE '{}'::jsonb
                       END AS details
                FROM authorization_audit_events event
                WHERE event.operator_id = $1
                  AND event.occurred_at >= $2 AND event.occurred_at < $3

                UNION ALL

                SELECT action.id, 'alert_action'::text AS source, action.actor_id,
                       action.action, 'alert'::text AS target_type,
                       action.alert_id::text AS target_reference, action.occurred_at,
                       jsonb_strip_nulls(jsonb_build_object(
                         'assigned_identity_id', action.assigned_identity_id,
                         'dismissal_reason', action.dismissal_reason)) AS details
                FROM alert_actions action
                WHERE action.operator_id = $1
                  AND action.occurred_at >= $2 AND action.occurred_at < $3
            )
            SELECT id, source, actor_id, action, target_type, target_reference,
                   occurred_at, details
            FROM audit_events
            ORDER BY occurred_at DESC, source, id DESC
            LIMIT $4
            "#,
        )
        .bind(operator_id.as_uuid())
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(&self.database)
        .await?;
        for event in &mut events {
            event.risk = classify_risk(event);
        }
        Ok(events)
    }
}

pub fn audit_router(store: AuditStore) -> Router {
    Router::new()
        .route("/api/admin/audit-events", get(list_events))
        .route("/api/admin/audit-events/export", get(export_events))
        .route("/api/admin/audit-alerts", get(list_signals))
        .with_state(store)
}

async fn list_events(
    State(store): State<AuditStore>,
    Extension(context): Extension<AuthContext>,
    Query(query): Query<ReviewQuery>,
) -> Result<Json<AuditEventList>, AuditError> {
    authorize(&context)?;
    let to = query.to.unwrap_or_else(Utc::now);
    let from = query
        .from
        .unwrap_or_else(|| to - Duration::hours(DEFAULT_REVIEW_HOURS));
    validate_range(from, to, None)?;
    let limit = query.limit.unwrap_or(100);
    if !(1..=250).contains(&limit) {
        return Err(AuditError::InvalidRange);
    }
    let data = store
        .events(context.operator_id, from, to, limit)
        .await
        .map_err(|_| AuditError::Unavailable)?;
    Ok(Json(AuditEventList { data, from, to }))
}

async fn export_events(
    State(store): State<AuditStore>,
    Extension(context): Extension<AuthContext>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, AuditError> {
    authorize(&context)?;
    validate_range(query.from, query.to, Some(Duration::days(MAX_EXPORT_DAYS)))?;
    let events = store
        .events(
            context.operator_id,
            query.from,
            query.to,
            MAX_EXPORT_EVENTS + 1,
        )
        .await
        .map_err(|_| AuditError::Unavailable)?;
    if events.len() as i64 > MAX_EXPORT_EVENTS {
        return Err(AuditError::ScopeTooLarge);
    }
    let csv = audit_csv(&events);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=flight-tracker-audit.csv"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((headers, csv).into_response())
}

async fn list_signals(
    State(store): State<AuditStore>,
    Extension(context): Extension<AuthContext>,
    Query(query): Query<MonitorQuery>,
) -> Result<Json<AuditSignalList>, AuditError> {
    authorize(&context)?;
    let through = Utc::now();
    let since = query
        .since
        .unwrap_or_else(|| through - Duration::hours(DEFAULT_REVIEW_HOURS));
    validate_range(since, through, Some(Duration::hours(DEFAULT_REVIEW_HOURS)))?;
    let events = store
        .events(context.operator_id, since, through, MAX_MONITOR_EVENTS + 1)
        .await
        .map_err(|_| AuditError::Unavailable)?;
    if events.len() as i64 > MAX_MONITOR_EVENTS {
        return Err(AuditError::ScopeTooLarge);
    }
    Ok(Json(AuditSignalList {
        data: detect_signals(&events),
        since,
        through,
    }))
}

fn authorize(context: &AuthContext) -> Result<(), AuditError> {
    require(context, Permission::ReviewAudit).map_err(|_| AuditError::Forbidden)
}

fn validate_range(
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    maximum: Option<Duration>,
) -> Result<(), AuditError> {
    if from >= to || maximum.is_some_and(|allowed| to - from > allowed) {
        return Err(AuditError::InvalidRange);
    }
    Ok(())
}

fn classify_risk(event: &AuditEventView) -> AuditRisk {
    match event.action.as_str() {
        "session.revoked" | "dismiss" | "resolve" => AuditRisk::High,
        "membership.updated"
            if event.details.get("status").and_then(Value::as_str) == Some("revoked")
                || event.details.get("role").and_then(Value::as_str) == Some("administrator") =>
        {
            AuditRisk::High
        }
        "membership.updated" | "assign" => AuditRisk::Sensitive,
        _ => AuditRisk::Routine,
    }
}

fn detect_signals(events: &[AuditEventView]) -> Vec<AuditSignalView> {
    let mut signals = events
        .iter()
        .filter(|event| event.risk == AuditRisk::High)
        .map(|event| AuditSignalView {
            code: "high_risk_action",
            severity: "warning",
            actor_id: event.actor_id.clone(),
            occurred_at: event.occurred_at,
            event_id: Some(event.id),
            message: format!("High-risk audit action recorded: {}", event.action),
        })
        .collect::<Vec<_>>();

    let mut by_actor: HashMap<&str, Vec<&AuditEventView>> = HashMap::new();
    for event in events
        .iter()
        .filter(|event| event.risk != AuditRisk::Routine)
    {
        by_actor.entry(&event.actor_id).or_default().push(event);
    }
    for (actor_id, mut actor_events) in by_actor {
        actor_events.sort_by_key(|event| event.occurred_at);
        let mut left = 0;
        for right in 0..actor_events.len() {
            while actor_events[right].occurred_at - actor_events[left].occurred_at
                > Duration::minutes(BURST_WINDOW_MINUTES)
            {
                left += 1;
            }
            if right + 1 - left == 3 {
                signals.push(AuditSignalView {
                    code: "privileged_action_burst",
                    severity: "critical",
                    actor_id: actor_id.to_owned(),
                    occurred_at: actor_events[right].occurred_at,
                    event_id: Some(actor_events[right].id),
                    message: format!(
                        "Three privileged actions occurred within {BURST_WINDOW_MINUTES} minutes"
                    ),
                });
                break;
            }
        }
    }
    signals.sort_by(|left, right| {
        right
            .occurred_at
            .cmp(&left.occurred_at)
            .then_with(|| left.code.cmp(right.code))
    });
    signals
}

fn audit_csv(events: &[AuditEventView]) -> String {
    let mut csv = String::from(
        "occurred_at,source,risk,actor_id,action,target_type,target_reference,details\r\n",
    );
    for event in events {
        let risk = match event.risk {
            AuditRisk::Routine => "routine",
            AuditRisk::Sensitive => "sensitive",
            AuditRisk::High => "high",
        };
        let values = [
            event.occurred_at.to_rfc3339(),
            event.source.clone(),
            risk.to_owned(),
            event.actor_id.clone(),
            event.action.clone(),
            event.target_type.clone(),
            event.target_reference.clone().unwrap_or_default(),
            event.details.to_string(),
        ];
        csv.push_str(
            &values
                .iter()
                .map(|value| csv_cell(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push_str("\r\n");
    }
    csv
}

fn csv_cell(value: &str) -> String {
    let safe = if value.starts_with(['=', '+', '-', '@']) {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: u128, actor: &str, action: &str, occurred_at: DateTime<Utc>) -> AuditEventView {
        let mut event = AuditEventView {
            id: Uuid::from_u128(id),
            source: "alert_action".into(),
            actor_id: actor.into(),
            action: action.into(),
            target_type: "alert".into(),
            target_reference: Some(Uuid::from_u128(99).to_string()),
            occurred_at,
            details: json!({}),
            risk: AuditRisk::Routine,
        };
        event.risk = classify_risk(&event);
        event
    }

    #[test]
    fn high_risk_actions_and_actor_bursts_are_deterministic() {
        let now = Utc::now();
        let events = vec![
            event(1, "actor-a", "resolve", now),
            event(2, "actor-a", "assign", now - Duration::minutes(2)),
            event(3, "actor-a", "dismiss", now - Duration::minutes(4)),
            event(4, "actor-b", "comment", now),
        ];
        let signals = detect_signals(&events);
        assert_eq!(
            signals
                .iter()
                .filter(|signal| signal.code == "high_risk_action")
                .count(),
            2
        );
        assert!(signals.iter().any(|signal| {
            signal.code == "privileged_action_burst" && signal.actor_id == "actor-a"
        }));
        assert!(!signals.iter().any(|signal| signal.actor_id == "actor-b"));
    }

    #[test]
    fn csv_is_formula_safe_and_contains_only_pre_redacted_fields() {
        let now = Utc::now();
        let events = vec![event(1, "=unsafe", "comment", now)];
        let csv = audit_csv(&events);
        assert!(csv.contains("\"'=unsafe\""));
        assert!(!csv.contains("comment body"));
        assert!(csv.ends_with("\r\n"));
    }

    #[test]
    fn export_and_monitor_ranges_are_bounded() {
        let now = Utc::now();
        assert!(validate_range(now, now, None).is_err());
        assert!(validate_range(now - Duration::days(32), now, Some(Duration::days(31))).is_err());
        assert!(validate_range(now - Duration::days(1), now, Some(Duration::days(31))).is_ok());
    }
}
