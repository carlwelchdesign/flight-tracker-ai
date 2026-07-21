use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{AlertActionKind, AlertLifecycle, AlertSeverity, OperatorId};

use super::{AlertCandidate, LifecycleError, transition_lifecycle};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CreateAlertResult {
    Created(Uuid),
    Duplicate(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AlertActionRequest {
    pub operator_id: OperatorId,
    pub action: AlertActionKind,
    pub actor_id: String,
    pub idempotency_key: String,
    pub expected_workflow_version: i32,
    pub comment: Option<String>,
    pub assigned_identity_id: Option<Uuid>,
    pub dismissal_reason: Option<DismissalReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DismissalReason {
    DuplicateAlert,
    StaleSourceData,
    IncorrectCorrelation,
    NotOperationallyRelevant,
    Other,
}

impl DismissalReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::DuplicateAlert => "duplicate_alert",
            Self::StaleSourceData => "stale_source_data",
            Self::IncorrectCorrelation => "incorrect_correlation",
            Self::NotOperationallyRelevant => "not_operationally_relevant",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertQueueFilter {
    pub include_terminal: bool,
    pub severity: Option<String>,
    pub lifecycle: Option<String>,
    pub flight: Option<String>,
    pub event_from: Option<DateTime<Utc>>,
    pub event_to: Option<DateTime<Utc>>,
    pub assignment: Option<AssignmentFilter>,
    pub limit: i64,
}

impl Default for AlertQueueFilter {
    fn default() -> Self {
        Self {
            include_terminal: false,
            severity: None,
            lifecycle: None,
            flight: None,
            event_from: None,
            event_to: None,
            assignment: None,
            limit: 200,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentFilter {
    Unassigned,
    Identity(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct AlertAssignee {
    pub identity_id: Uuid,
    pub subject: String,
    pub display_name: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct AlertQueueItem {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub event_time: DateTime<Utc>,
    pub flight_id: Option<Uuid>,
    pub flight_callsign: Option<String>,
    pub hazard_id: Option<Uuid>,
    pub alert_type: String,
    pub severity: String,
    pub lifecycle: String,
    pub rule_id: String,
    pub rule_version: i32,
    pub series_key: String,
    pub alert_revision: i32,
    pub supersedes_alert_id: Option<Uuid>,
    pub attention_score: i16,
    pub score_version: i32,
    pub workflow_version: i32,
    pub assigned_identity_id: Option<Uuid>,
    pub assigned_subject: Option<String>,
    pub assigned_display_name: Option<String>,
    pub evidence: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct AlertActionView {
    pub id: Uuid,
    pub action: String,
    pub actor_id: String,
    pub occurred_at: DateTime<Utc>,
    pub comment: Option<String>,
    pub assigned_identity_id: Option<Uuid>,
    pub dismissal_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlertDetail {
    #[serde(flatten)]
    pub alert: AlertQueueItem,
    pub actions: Vec<AlertActionView>,
}

#[derive(Debug, Error)]
pub enum AlertStoreError {
    #[error("alert persistence failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("alert was not found")]
    NotFound,
    #[error("actor_id and idempotency_key must not be empty")]
    InvalidActionIdentity,
    #[error("idempotency_key was already used for a different alert")]
    IdempotencyConflict,
    #[error("alert changed since it was loaded; refresh and review the latest workflow state")]
    ConcurrentModification,
    #[error("the selected assignee is not an active alert manager for this tenant")]
    InvalidAssignee,
    #[error("dismiss requires a structured reason; other also requires an explanation")]
    InvalidDismissalReason,
    #[error("comment requires a non-empty dispatcher note")]
    InvalidComment,
    #[error(transparent)]
    Lifecycle(#[from] LifecycleError),
    #[error("stored alert lifecycle is invalid")]
    InvalidStoredLifecycle,
    #[error("alert series exhausted its supported revision range")]
    AlertRevisionExhausted,
}

#[derive(Clone)]
pub struct AlertStore {
    database: PgPool,
}

impl AlertStore {
    pub fn new(database: PgPool) -> Self {
        Self { database }
    }

    pub async fn create_from_candidate(
        &self,
        candidate: &AlertCandidate,
        now: DateTime<Utc>,
    ) -> Result<CreateAlertResult, AlertStoreError> {
        let mut transaction = self.database.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(&candidate.series_key)
            .execute(&mut *transaction)
            .await?;

        if let Some(id) = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM alerts WHERE operator_id = $1 AND dedupe_key = $2",
        )
        .bind(candidate.operator_id.as_uuid())
        .bind(&candidate.dedupe_key)
        .fetch_optional(&mut *transaction)
        .await?
        {
            transaction.rollback().await?;
            return Ok(CreateAlertResult::Duplicate(id));
        }

        if let Some(id) = sqlx::query_scalar::<_, Uuid>(
            "SELECT alert_id FROM alert_history_tombstones WHERE operator_id = $1 AND dedupe_key = $2",
        )
        .bind(candidate.operator_id.as_uuid())
        .bind(&candidate.dedupe_key)
        .fetch_optional(&mut *transaction)
        .await?
        {
            transaction.rollback().await?;
            return Ok(CreateAlertResult::Duplicate(id));
        }

        let previous = sqlx::query_as::<_, (Uuid, i32, String)>(
            r#"
            SELECT id, alert_revision, lifecycle
            FROM alerts
            WHERE operator_id = $1 AND series_key = $2
            ORDER BY alert_revision DESC
            LIMIT 1
            FOR UPDATE
            "#,
        )
        .bind(candidate.operator_id.as_uuid())
        .bind(&candidate.series_key)
        .fetch_optional(&mut *transaction)
        .await?;
        let deleted_revision = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(alert_revision) FROM alert_history_tombstones WHERE operator_id = $1 AND series_key = $2",
        )
        .bind(candidate.operator_id.as_uuid())
        .bind(&candidate.series_key)
        .fetch_one(&mut *transaction)
        .await?
        .unwrap_or(0);
        let current_revision = previous
            .as_ref()
            .map_or(0, |(_, revision, _)| *revision)
            .max(deleted_revision);
        let alert_revision = current_revision
            .checked_add(1)
            .ok_or(AlertStoreError::AlertRevisionExhausted)?;
        let id = Uuid::new_v5(
            &candidate.operator_id.as_uuid(),
            candidate.dedupe_key.as_bytes(),
        );
        let evidence = json!({
            "attention": candidate.attention,
            "route_hazard": candidate.decision.evidence,
        });

        sqlx::query(
            r#"
            INSERT INTO alerts (
                id, operator_id, schema_version, event_time, received_at, processed_at,
                flight_id, hazard_id, alert_type, severity, lifecycle, rule_id,
                rule_version, dedupe_key, series_key, alert_revision,
                supersedes_alert_id, attention_score, score_version, evidence
            ) VALUES (
                $1, $2, 1, $3, $3, $3, $4, $5, $6, $7, 'open',
                $8, $9, $10, $11, $12, $13, $14, $15, $16
            )
            "#,
        )
        .bind(id)
        .bind(candidate.operator_id.as_uuid())
        .bind(now)
        .bind(candidate.flight_id.as_uuid())
        .bind(candidate.hazard_id.as_uuid())
        .bind(&candidate.alert_type)
        .bind(severity_name(candidate.severity))
        .bind(&candidate.decision.evidence.rule_id)
        .bind(i32::try_from(candidate.decision.evidence.rule_version).unwrap_or(i32::MAX))
        .bind(&candidate.dedupe_key)
        .bind(&candidate.series_key)
        .bind(alert_revision)
        .bind(previous.as_ref().map(|(id, _, _)| *id))
        .bind(i16::from(candidate.attention.total))
        .bind(i32::try_from(candidate.attention.score_version).unwrap_or(i32::MAX))
        .bind(evidence)
        .execute(&mut *transaction)
        .await?;

        for (ordinal, envelope_id) in candidate.evidence_envelope_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO alert_evidence (operator_id, alert_id, source_envelope_id, ordinal) VALUES ($1, $2, $3, $4)",
            )
            .bind(candidate.operator_id.as_uuid())
            .bind(id)
            .bind(envelope_id.as_uuid())
            .bind(i32::try_from(ordinal).unwrap_or(i32::MAX))
            .execute(&mut *transaction)
            .await?;
        }

        if let Some((previous_id, _, lifecycle)) = previous
            && matches!(lifecycle.as_str(), "open" | "acknowledged")
        {
            sqlx::query(
                "UPDATE alerts SET lifecycle = 'resolved', workflow_version = workflow_version + 1 WHERE operator_id = $1 AND id = $2",
            )
            .bind(candidate.operator_id.as_uuid())
            .bind(previous_id)
            .execute(&mut *transaction)
            .await?;
            insert_action(
                &mut transaction,
                candidate.operator_id.as_uuid(),
                previous_id,
                AlertActionKind::Resolve,
                "system:alert-supersession",
                &format!("superseded-by:{id}"),
                Some("Superseded by newer material evidence"),
                None,
                None,
                now,
            )
            .await?;
        }

        transaction.commit().await?;
        Ok(CreateAlertResult::Created(id))
    }

    pub async fn list_queue(
        &self,
        operator_id: OperatorId,
        filter: &AlertQueueFilter,
    ) -> Result<Vec<AlertQueueItem>, AlertStoreError> {
        let (filter_assignment, unassigned, assigned_identity_id) = match filter.assignment {
            None => (false, false, None),
            Some(AssignmentFilter::Unassigned) => (true, true, None),
            Some(AssignmentFilter::Identity(identity_id)) => (true, false, Some(identity_id)),
        };
        let rows = sqlx::query_as::<_, AlertQueueItem>(
            r#"
            SELECT current_alert.id, current_alert.operator_id, current_alert.event_time,
                   current_alert.flight_id, current_alert.hazard_id, current_alert.alert_type,
                   flight.callsign AS flight_callsign,
                   current_alert.severity, current_alert.lifecycle, current_alert.rule_id,
                   current_alert.rule_version, current_alert.series_key,
                   current_alert.alert_revision, current_alert.supersedes_alert_id,
                   current_alert.attention_score, current_alert.score_version,
                   current_alert.workflow_version, current_alert.assigned_identity_id,
                   assigned.subject AS assigned_subject,
                   assigned.display_name AS assigned_display_name,
                   current_alert.evidence
            FROM alerts current_alert
            LEFT JOIN auth_identities assigned ON assigned.id = current_alert.assigned_identity_id
            LEFT JOIN flights flight
              ON flight.operator_id = current_alert.operator_id
             AND flight.id = current_alert.flight_id
            WHERE current_alert.operator_id = $1
              AND ($2 OR $4::text IS NOT NULL OR current_alert.lifecycle IN ('open', 'acknowledged'))
              AND ($3::text IS NULL OR current_alert.severity = $3)
              AND ($4::text IS NULL OR current_alert.lifecycle = $4)
              AND (
                  $5::text IS NULL
                  OR current_alert.flight_id::text = $5
                  OR lower(flight.callsign) = lower($5)
              )
              AND ($6::timestamptz IS NULL OR current_alert.event_time >= $6)
              AND ($7::timestamptz IS NULL OR current_alert.event_time <= $7)
              AND (
                  NOT $8
                  OR ($9 AND current_alert.assigned_identity_id IS NULL)
                  OR (NOT $9 AND current_alert.assigned_identity_id = $10)
              )
              AND NOT EXISTS (
                  SELECT 1 FROM alerts newer
                  WHERE newer.operator_id = current_alert.operator_id
                    AND newer.supersedes_alert_id = current_alert.id
              )
            ORDER BY
                CASE current_alert.lifecycle WHEN 'open' THEN 0 WHEN 'acknowledged' THEN 1 ELSE 2 END,
                current_alert.attention_score DESC,
                current_alert.event_time ASC,
                current_alert.id ASC
            LIMIT $11
            "#,
        )
        .bind(operator_id.as_uuid())
        .bind(filter.include_terminal)
        .bind(filter.severity.as_deref())
        .bind(filter.lifecycle.as_deref())
        .bind(filter.flight.as_deref())
        .bind(filter.event_from)
        .bind(filter.event_to)
        .bind(filter_assignment)
        .bind(unassigned)
        .bind(assigned_identity_id)
        .bind(filter.limit.clamp(1, 500))
        .fetch_all(&self.database)
        .await?;
        Ok(rows)
    }

    pub async fn list_assignees(
        &self,
        operator_id: OperatorId,
    ) -> Result<Vec<AlertAssignee>, AlertStoreError> {
        Ok(sqlx::query_as::<_, AlertAssignee>(
            r#"
            SELECT identity.id AS identity_id, identity.subject, identity.display_name,
                   membership.role
            FROM operator_memberships membership
            JOIN auth_identities identity ON identity.id = membership.identity_id
            WHERE membership.operator_id = $1
              AND membership.status = 'active'
              AND identity.disabled_at IS NULL
              AND membership.role IN ('dispatcher', 'operator', 'administrator')
            ORDER BY identity.display_name NULLS LAST, identity.subject, identity.id
            "#,
        )
        .bind(operator_id.as_uuid())
        .fetch_all(&self.database)
        .await?)
    }

    pub async fn detail(
        &self,
        operator_id: OperatorId,
        alert_id: Uuid,
    ) -> Result<AlertDetail, AlertStoreError> {
        let alert = fetch_alert(&self.database, operator_id.as_uuid(), alert_id)
            .await?
            .ok_or(AlertStoreError::NotFound)?;
        let actions = sqlx::query_as::<_, AlertActionView>(
            r#"
            SELECT id, action, actor_id, occurred_at, comment,
                   assigned_identity_id, dismissal_reason
            FROM alert_actions
            WHERE operator_id = $1 AND alert_id = $2
            ORDER BY occurred_at, id
            "#,
        )
        .bind(operator_id.as_uuid())
        .bind(alert_id)
        .fetch_all(&self.database)
        .await?;
        Ok(AlertDetail { alert, actions })
    }

    pub async fn apply_action(
        &self,
        alert_id: Uuid,
        request: &AlertActionRequest,
        now: DateTime<Utc>,
    ) -> Result<AlertDetail, AlertStoreError> {
        if request.actor_id.trim().is_empty() || request.idempotency_key.trim().is_empty() {
            return Err(AlertStoreError::InvalidActionIdentity);
        }
        let mut transaction = self.database.begin().await?;
        let lock_key = format!(
            "{}:{}",
            request.operator_id.as_uuid(),
            request.idempotency_key
        );
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(lock_key)
            .execute(&mut *transaction)
            .await?;
        if let Some(existing_alert_id) = sqlx::query_scalar::<_, Uuid>(
            "SELECT alert_id FROM alert_actions WHERE operator_id = $1 AND idempotency_key = $2",
        )
        .bind(request.operator_id.as_uuid())
        .bind(&request.idempotency_key)
        .fetch_optional(&mut *transaction)
        .await?
        {
            transaction.rollback().await?;
            if existing_alert_id != alert_id {
                return Err(AlertStoreError::IdempotencyConflict);
            }
            return self.detail(request.operator_id, alert_id).await;
        }
        let (lifecycle, workflow_version) = sqlx::query_as::<_, (String, i32)>(
            "SELECT lifecycle, workflow_version FROM alerts WHERE operator_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(request.operator_id.as_uuid())
        .bind(alert_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(AlertStoreError::NotFound)?;
        if workflow_version != request.expected_workflow_version {
            return Err(AlertStoreError::ConcurrentModification);
        }
        validate_action_request(&mut transaction, request).await?;
        let current = parse_lifecycle(&lifecycle)?;
        let lifecycle_reason = request
            .dismissal_reason
            .map(DismissalReason::as_str)
            .or(request.comment.as_deref());
        let next = transition_lifecycle(current, request.action, lifecycle_reason)?;

        insert_action(
            &mut transaction,
            request.operator_id.as_uuid(),
            alert_id,
            request.action,
            request.actor_id.trim(),
            request.idempotency_key.trim(),
            request.comment.as_deref().map(str::trim),
            request.assigned_identity_id,
            request.dismissal_reason,
            now,
        )
        .await?;
        sqlx::query(
            r#"
            UPDATE alerts
            SET lifecycle = $3,
                workflow_version = workflow_version + 1,
                assigned_identity_id = CASE WHEN $4 = 'assign' THEN $5 ELSE assigned_identity_id END,
                assigned_at = CASE WHEN $4 = 'assign' THEN $6 ELSE assigned_at END,
                assigned_by_actor_id = CASE WHEN $4 = 'assign' THEN $7 ELSE assigned_by_actor_id END
            WHERE operator_id = $1 AND id = $2
            "#,
        )
        .bind(request.operator_id.as_uuid())
        .bind(alert_id)
        .bind(lifecycle_name(next))
        .bind(action_name(request.action))
        .bind(request.assigned_identity_id)
        .bind(now)
        .bind(request.actor_id.trim())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.detail(request.operator_id, alert_id).await
    }
}

async fn validate_action_request(
    transaction: &mut Transaction<'_, Postgres>,
    request: &AlertActionRequest,
) -> Result<(), AlertStoreError> {
    match request.action {
        AlertActionKind::Assign => {
            let Some(identity_id) = request.assigned_identity_id else {
                return Err(AlertStoreError::InvalidAssignee);
            };
            let valid = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM operator_memberships membership
                    JOIN auth_identities identity ON identity.id = membership.identity_id
                    WHERE membership.operator_id = $1
                      AND membership.identity_id = $2
                      AND membership.status = 'active'
                      AND identity.disabled_at IS NULL
                      AND membership.role IN ('dispatcher', 'operator', 'administrator')
                )
                "#,
            )
            .bind(request.operator_id.as_uuid())
            .bind(identity_id)
            .fetch_one(&mut **transaction)
            .await?;
            if !valid {
                return Err(AlertStoreError::InvalidAssignee);
            }
        }
        AlertActionKind::Dismiss => {
            let Some(reason) = request.dismissal_reason else {
                return Err(AlertStoreError::InvalidDismissalReason);
            };
            if reason == DismissalReason::Other
                && request
                    .comment
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err(AlertStoreError::InvalidDismissalReason);
            }
        }
        AlertActionKind::Comment
            if request
                .comment
                .as_deref()
                .is_none_or(|value| value.trim().is_empty()) =>
        {
            return Err(AlertStoreError::InvalidComment);
        }
        _ if request.assigned_identity_id.is_some() || request.dismissal_reason.is_some() => {
            return Err(AlertStoreError::InvalidActionIdentity);
        }
        _ => {}
    }
    Ok(())
}

async fn fetch_alert(
    database: &PgPool,
    operator_id: Uuid,
    alert_id: Uuid,
) -> Result<Option<AlertQueueItem>, sqlx::Error> {
    sqlx::query_as::<_, AlertQueueItem>(
        r#"
        SELECT alert.id, alert.operator_id, alert.event_time, alert.flight_id,
               flight.callsign AS flight_callsign,
               alert.hazard_id, alert.alert_type, alert.severity, alert.lifecycle,
               alert.rule_id, alert.rule_version, alert.series_key,
               alert.alert_revision, alert.supersedes_alert_id, alert.attention_score,
               alert.score_version, alert.workflow_version, alert.assigned_identity_id,
               assigned.subject AS assigned_subject,
               assigned.display_name AS assigned_display_name, alert.evidence
        FROM alerts alert
        LEFT JOIN auth_identities assigned ON assigned.id = alert.assigned_identity_id
        LEFT JOIN flights flight
          ON flight.operator_id = alert.operator_id AND flight.id = alert.flight_id
        WHERE alert.operator_id = $1 AND alert.id = $2
        "#,
    )
    .bind(operator_id)
    .bind(alert_id)
    .fetch_optional(database)
    .await
}

#[allow(clippy::too_many_arguments)]
async fn insert_action(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: Uuid,
    alert_id: Uuid,
    action: AlertActionKind,
    actor_id: &str,
    idempotency_key: &str,
    comment: Option<&str>,
    assigned_identity_id: Option<Uuid>,
    dismissal_reason: Option<DismissalReason>,
    occurred_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v5(&operator_id, idempotency_key.as_bytes());
    sqlx::query(
        r#"
        INSERT INTO alert_actions (
            id, operator_id, alert_id, schema_version, action, actor_id,
            occurred_at, comment, idempotency_key, assigned_identity_id,
            dismissal_reason
        ) VALUES ($1, $2, $3, 1, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(id)
    .bind(operator_id)
    .bind(alert_id)
    .bind(action_name(action))
    .bind(actor_id)
    .bind(occurred_at)
    .bind(comment)
    .bind(idempotency_key)
    .bind(assigned_identity_id)
    .bind(dismissal_reason.map(DismissalReason::as_str))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn parse_lifecycle(value: &str) -> Result<AlertLifecycle, AlertStoreError> {
    match value {
        "open" => Ok(AlertLifecycle::Open),
        "acknowledged" => Ok(AlertLifecycle::Acknowledged),
        "dismissed" => Ok(AlertLifecycle::Dismissed),
        "resolved" => Ok(AlertLifecycle::Resolved),
        _ => Err(AlertStoreError::InvalidStoredLifecycle),
    }
}

fn lifecycle_name(value: AlertLifecycle) -> &'static str {
    match value {
        AlertLifecycle::Open => "open",
        AlertLifecycle::Acknowledged => "acknowledged",
        AlertLifecycle::Dismissed => "dismissed",
        AlertLifecycle::Resolved => "resolved",
    }
}

fn action_name(value: AlertActionKind) -> &'static str {
    match value {
        AlertActionKind::Acknowledge => "acknowledge",
        AlertActionKind::Assign => "assign",
        AlertActionKind::Dismiss => "dismiss",
        AlertActionKind::Comment => "comment",
        AlertActionKind::Resolve => "resolve",
    }
}

fn severity_name(value: AlertSeverity) -> &'static str {
    match value {
        AlertSeverity::Information => "information",
        AlertSeverity::Advisory => "advisory",
        AlertSeverity::Warning => "warning",
        AlertSeverity::Critical => "critical",
    }
}
