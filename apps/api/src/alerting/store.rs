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
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct AlertQueueItem {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub event_time: DateTime<Utc>,
    pub flight_id: Option<Uuid>,
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
    pub evidence: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct AlertActionView {
    pub id: Uuid,
    pub action: String,
    pub actor_id: String,
    pub occurred_at: DateTime<Utc>,
    pub comment: Option<String>,
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
    #[error(transparent)]
    Lifecycle(#[from] LifecycleError),
    #[error("stored alert lifecycle is invalid")]
    InvalidStoredLifecycle,
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
        let alert_revision = previous.as_ref().map_or(1, |(_, revision, _)| revision + 1);
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
                "UPDATE alerts SET lifecycle = 'resolved' WHERE operator_id = $1 AND id = $2",
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
        include_terminal: bool,
    ) -> Result<Vec<AlertQueueItem>, AlertStoreError> {
        let rows = sqlx::query_as::<_, AlertQueueItem>(
            r#"
            SELECT id, operator_id, event_time, flight_id, hazard_id, alert_type,
                   severity, lifecycle, rule_id, rule_version, series_key,
                   alert_revision, supersedes_alert_id, attention_score,
                   score_version, evidence
            FROM alerts current_alert
            WHERE operator_id = $1
              AND ($2 OR lifecycle IN ('open', 'acknowledged'))
              AND NOT EXISTS (
                  SELECT 1 FROM alerts newer
                  WHERE newer.operator_id = current_alert.operator_id
                    AND newer.supersedes_alert_id = current_alert.id
              )
            ORDER BY
                CASE lifecycle WHEN 'open' THEN 0 WHEN 'acknowledged' THEN 1 ELSE 2 END,
                attention_score DESC,
                event_time ASC,
                id ASC
            "#,
        )
        .bind(operator_id.as_uuid())
        .bind(include_terminal)
        .fetch_all(&self.database)
        .await?;
        Ok(rows)
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
            SELECT id, action, actor_id, occurred_at, comment
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
        let lifecycle = sqlx::query_scalar::<_, String>(
            "SELECT lifecycle FROM alerts WHERE operator_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(request.operator_id.as_uuid())
        .bind(alert_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(AlertStoreError::NotFound)?;
        let current = parse_lifecycle(&lifecycle)?;
        let next = transition_lifecycle(current, request.action, request.comment.as_deref())?;

        insert_action(
            &mut transaction,
            request.operator_id.as_uuid(),
            alert_id,
            request.action,
            request.actor_id.trim(),
            request.idempotency_key.trim(),
            request.comment.as_deref().map(str::trim),
            now,
        )
        .await?;
        if next != current {
            sqlx::query("UPDATE alerts SET lifecycle = $3 WHERE operator_id = $1 AND id = $2")
                .bind(request.operator_id.as_uuid())
                .bind(alert_id)
                .bind(lifecycle_name(next))
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        self.detail(request.operator_id, alert_id).await
    }
}

async fn fetch_alert(
    database: &PgPool,
    operator_id: Uuid,
    alert_id: Uuid,
) -> Result<Option<AlertQueueItem>, sqlx::Error> {
    sqlx::query_as::<_, AlertQueueItem>(
        r#"
        SELECT id, operator_id, event_time, flight_id, hazard_id, alert_type,
               severity, lifecycle, rule_id, rule_version, series_key,
               alert_revision, supersedes_alert_id, attention_score,
               score_version, evidence
        FROM alerts WHERE operator_id = $1 AND id = $2
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
    occurred_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v5(&operator_id, idempotency_key.as_bytes());
    sqlx::query(
        r#"
        INSERT INTO alert_actions (
            id, operator_id, alert_id, schema_version, action, actor_id,
            occurred_at, comment, idempotency_key
        ) VALUES ($1, $2, $3, 1, $4, $5, $6, $7, $8)
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
