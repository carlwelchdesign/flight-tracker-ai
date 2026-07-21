use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, Permission, require},
    domain::OperatorId,
};

mod integrity;
mod schedule;

pub use integrity::{
    RetentionIntegrityView, RetentionIntegrityViolations, RetentionTombstoneCounts,
};
pub use schedule::{
    CreateRetentionSchedule, RetentionScheduleAttemptView, RetentionScheduleView,
    spawn_retention_scheduler,
};

const MIN_RETENTION_SECONDS: i64 = 3_600;
const MAX_RETENTION_SECONDS: i64 = 315_576_000;
const MAX_DELETE_RECORDS: i64 = 10_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionDataClass {
    #[default]
    ProviderRawPayload,
    AuthorizationAudit,
    SessionRevocation,
    IdentityMapping,
    TerminalAlertHistory,
    NormalizedOperationalFact,
}

impl RetentionDataClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProviderRawPayload => "provider_raw_payload",
            Self::AuthorizationAudit => "authorization_audit",
            Self::SessionRevocation => "session_revocation",
            Self::IdentityMapping => "identity_mapping",
            Self::TerminalAlertHistory => "terminal_alert_history",
            Self::NormalizedOperationalFact => "normalized_operational_fact",
        }
    }

    const fn inventory_key(self) -> &'static str {
        match self {
            Self::ProviderRawPayload => "provider_envelopes",
            Self::AuthorizationAudit => "authorization_audit_events",
            Self::SessionRevocation => "auth_session_revocations",
            Self::IdentityMapping => "auth_identities_minimized",
            Self::TerminalAlertHistory => "alerts",
            Self::NormalizedOperationalFact => "normalized_operational_facts",
        }
    }
}

impl TryFrom<&str> for RetentionDataClass {
    type Error = RetentionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "provider_raw_payload" => Ok(Self::ProviderRawPayload),
            "authorization_audit" => Ok(Self::AuthorizationAudit),
            "session_revocation" => Ok(Self::SessionRevocation),
            "identity_mapping" => Ok(Self::IdentityMapping),
            "terminal_alert_history" => Ok(Self::TerminalAlertHistory),
            "normalized_operational_fact" => Ok(Self::NormalizedOperationalFact),
            _ => Err(RetentionError::InvalidConfiguration),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateRetentionPolicy {
    #[serde(default)]
    pub data_class: RetentionDataClass,
    pub provider: String,
    pub retention_seconds: i64,
    pub approval_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PreviewRetentionRun {
    pub policy_id: Uuid,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct RetentionPolicyView {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub data_class: String,
    pub provider: String,
    pub version: i32,
    pub retention_seconds: i64,
    pub status: String,
    pub approval_reference: String,
    pub created_by_identity_id: Uuid,
    pub approved_by_identity_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub retired_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct RetentionRunView {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub policy_id: Uuid,
    pub policy_version: i32,
    pub data_class: String,
    pub provider: String,
    pub cutoff_at: DateTime<Utc>,
    pub status: String,
    pub preview_counts: Value,
    pub preview_fingerprint: Option<String>,
    pub deletion_counts: Option<Value>,
    pub requested_by_identity_id: Uuid,
    pub approved_by_identity_id: Option<Uuid>,
    pub executed_by_identity_id: Option<Uuid>,
    pub evidence_reference: String,
    pub requested_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Error)]
pub enum RetentionError {
    #[error("retention persistence failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("retention policy or run was not found in the current tenant")]
    NotFound,
    #[error("retention configuration is invalid")]
    InvalidConfiguration,
    #[error("a different administrator must approve this retention change")]
    SeparationOfDuties,
    #[error("retention policy or run is not in the required state")]
    InvalidState,
    #[error("retention inventory changed after preview; create and approve a new preview")]
    InventoryChanged,
    #[error("retention scope exceeds the maximum safe batch size")]
    ScopeTooLarge,
}

#[derive(Clone)]
pub struct RetentionStore {
    database: PgPool,
}

#[derive(sqlx::FromRow)]
struct EligibleEnvelope {
    id: Uuid,
    feed: String,
    raw_payload_sha256: String,
}

#[derive(sqlx::FromRow)]
struct AlertHistoryInventory {
    alerts: i64,
    alert_actions: i64,
    alert_evidence: i64,
}

#[derive(sqlx::FromRow)]
struct EligibleAlert {
    id: Uuid,
    dedupe_key: String,
    series_key: String,
    alert_revision: i32,
}

#[derive(sqlx::FromRow)]
struct OperationalFactInventory {
    airport_observations: i64,
    flights: i64,
    aircraft_positions: i64,
    planned_routes: i64,
    weather_hazards: i64,
}

#[derive(sqlx::FromRow)]
struct EligibleOperationalFact {
    fact_type: String,
    record_id: Uuid,
}

const OPERATIONAL_FACT_INVENTORY_SQL: &str = r#"
    SELECT
        COUNT(*) FILTER (WHERE fact_type = 'airport_observations') AS airport_observations,
        COUNT(*) FILTER (WHERE fact_type = 'flights') AS flights,
        COUNT(*) FILTER (WHERE fact_type = 'aircraft_positions') AS aircraft_positions,
        COUNT(*) FILTER (WHERE fact_type = 'planned_routes') AS planned_routes,
        COUNT(*) FILTER (WHERE fact_type = 'weather_hazards') AS weather_hazards
    FROM eligible_normalized_fact_ids($1, $2, $3)
    "#;

const ALERT_HISTORY_INVENTORY_SQL: &str = r#"
    WITH eligible_series AS (
        SELECT alert.series_key
        FROM alerts alert
        LEFT JOIN alert_actions action
          ON action.operator_id = alert.operator_id AND action.alert_id = alert.id
        WHERE alert.operator_id = $1
        GROUP BY alert.series_key
        HAVING BOOL_AND(alert.lifecycle IN ('dismissed', 'resolved'))
           AND MAX(GREATEST(
               alert.event_time,
               alert.received_at,
               alert.processed_at,
               COALESCE(action.occurred_at, alert.processed_at)
           )) < $2
    ), eligible_alerts AS (
        SELECT alert.id
        FROM alerts alert
        JOIN eligible_series series ON series.series_key = alert.series_key
        WHERE alert.operator_id = $1
    )
    SELECT
        (SELECT COUNT(*) FROM eligible_alerts) AS alerts,
        (SELECT COUNT(*) FROM alert_actions action
         JOIN eligible_alerts alert ON alert.id = action.alert_id
         WHERE action.operator_id = $1) AS alert_actions,
        (SELECT COUNT(*) FROM alert_evidence evidence
         JOIN eligible_alerts alert ON alert.id = evidence.alert_id
         WHERE evidence.operator_id = $1) AS alert_evidence
    "#;

impl RetentionStore {
    pub fn new(database: PgPool) -> Self {
        Self { database }
    }

    pub async fn list_policies(
        &self,
        operator_id: OperatorId,
    ) -> Result<Vec<RetentionPolicyView>, RetentionError> {
        Ok(sqlx::query_as::<_, RetentionPolicyView>(
            r#"
            SELECT id, operator_id, data_class, provider, version, retention_seconds,
                   status, approval_reference, created_by_identity_id,
                   approved_by_identity_id, created_at, approved_at, retired_at
            FROM retention_policies
            WHERE operator_id = $1
            ORDER BY data_class, provider, version DESC
            "#,
        )
        .bind(operator_id.as_uuid())
        .fetch_all(&self.database)
        .await?)
    }

    pub async fn create_policy(
        &self,
        actor: &AuthContext,
        request: &CreateRetentionPolicy,
        now: DateTime<Utc>,
    ) -> Result<RetentionPolicyView, RetentionError> {
        let provider = bounded_name(&request.provider)?;
        if !matches!(
            request.data_class,
            RetentionDataClass::ProviderRawPayload | RetentionDataClass::NormalizedOperationalFact
        ) && provider != "application"
        {
            return Err(RetentionError::InvalidConfiguration);
        }
        let data_class = request.data_class.as_str();
        let approval_reference = bounded_reference(&request.approval_reference)?;
        if !(MIN_RETENTION_SECONDS..=MAX_RETENTION_SECONDS).contains(&request.retention_seconds) {
            return Err(RetentionError::InvalidConfiguration);
        }
        let mut transaction = self.database.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!(
                "retention-policy:{}:{data_class}:{provider}",
                actor.operator_id.as_uuid()
            ))
            .execute(&mut *transaction)
            .await?;
        let version = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COALESCE(MAX(version), 0) + 1
            FROM retention_policies
            WHERE operator_id = $1 AND data_class = $2 AND provider = $3
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(data_class)
        .bind(&provider)
        .fetch_one(&mut *transaction)
        .await?;
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO retention_policies (
                id, operator_id, data_class, provider, version, retention_seconds,
                status, approval_reference, created_by_identity_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,'draft',$7,$8,$9)
            "#,
        )
        .bind(id)
        .bind(actor.operator_id.as_uuid())
        .bind(data_class)
        .bind(provider)
        .bind(version)
        .bind(request.retention_seconds)
        .bind(approval_reference)
        .bind(actor.identity_id)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.policy(actor.operator_id, id).await
    }

    pub async fn approve_policy(
        &self,
        actor: &AuthContext,
        policy_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RetentionPolicyView, RetentionError> {
        let mut transaction = self.database.begin().await?;
        let policy = lock_policy(&mut transaction, actor.operator_id, policy_id).await?;
        if policy.status != "draft" {
            return Err(RetentionError::InvalidState);
        }
        if policy.created_by_identity_id == actor.identity_id {
            return Err(RetentionError::SeparationOfDuties);
        }
        sqlx::query(
            r#"
            UPDATE retention_policies
            SET status = 'retired', retired_at = $4
            WHERE operator_id = $1 AND data_class = $2 AND provider = $3
              AND status = 'approved'
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(&policy.data_class)
        .bind(&policy.provider)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE retention_schedules schedule
            SET status = 'retired', paused_at = $4
            FROM retention_policies policy
            WHERE schedule.operator_id = $1
              AND schedule.status = 'active'
              AND schedule.policy_id = policy.id
              AND policy.operator_id = $1
              AND policy.data_class = $2
              AND policy.provider = $3
              AND policy.status = 'retired'
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(&policy.data_class)
        .bind(&policy.provider)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE retention_policies
            SET status = 'approved', approved_by_identity_id = $3, approved_at = $4
            WHERE operator_id = $1 AND id = $2
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(policy_id)
        .bind(actor.identity_id)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.policy(actor.operator_id, policy_id).await
    }

    pub async fn preview_run(
        &self,
        actor: &AuthContext,
        request: &PreviewRetentionRun,
        now: DateTime<Utc>,
    ) -> Result<RetentionRunView, RetentionError> {
        let evidence_reference = bounded_reference(&request.evidence_reference)?;
        let mut transaction = self.database.begin().await?;
        set_repeatable_read(&mut transaction).await?;
        let policy = lock_policy(&mut transaction, actor.operator_id, request.policy_id).await?;
        if policy.status != "approved" {
            return Err(RetentionError::InvalidState);
        }
        let cutoff_at = now - Duration::seconds(policy.retention_seconds);
        let data_class = RetentionDataClass::try_from(policy.data_class.as_str())?;
        let preview_counts = inventory_counts_transaction(
            &mut transaction,
            actor.operator_id,
            data_class,
            &policy.provider,
            cutoff_at,
        )
        .await?;
        if inventory_total(&preview_counts)? > MAX_DELETE_RECORDS {
            return Err(RetentionError::ScopeTooLarge);
        }
        let preview_fingerprint = inventory_fingerprint_transaction(
            &mut transaction,
            actor.operator_id,
            data_class,
            &policy.provider,
            cutoff_at,
        )
        .await?;
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO retention_runs (
                id, operator_id, policy_id, policy_version, data_class, provider,
                cutoff_at, status, preview_counts, preview_fingerprint,
                requested_by_identity_id, evidence_reference, requested_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,'awaiting_approval',$8,$9,$10,$11,$12)
            "#,
        )
        .bind(id)
        .bind(actor.operator_id.as_uuid())
        .bind(policy.id)
        .bind(policy.version)
        .bind(&policy.data_class)
        .bind(&policy.provider)
        .bind(cutoff_at)
        .bind(&preview_counts)
        .bind(&preview_fingerprint)
        .bind(actor.identity_id)
        .bind(evidence_reference)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.run(actor.operator_id, id).await
    }

    pub async fn approve_run(
        &self,
        actor: &AuthContext,
        run_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RetentionRunView, RetentionError> {
        let run = self.run(actor.operator_id, run_id).await?;
        if run.status != "awaiting_approval" {
            return Err(RetentionError::InvalidState);
        }
        if run.requested_by_identity_id == actor.identity_id {
            return Err(RetentionError::SeparationOfDuties);
        }
        let updated = sqlx::query(
            r#"
            UPDATE retention_runs
            SET status = 'approved', approved_by_identity_id = $3, approved_at = $4
            WHERE operator_id = $1 AND id = $2 AND status = 'awaiting_approval'
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(run_id)
        .bind(actor.identity_id)
        .bind(now)
        .execute(&self.database)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(RetentionError::InvalidState);
        }
        self.run(actor.operator_id, run_id).await
    }

    pub async fn execute_run(
        &self,
        actor: &AuthContext,
        run_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RetentionRunView, RetentionError> {
        let mut transaction = self.database.begin().await?;
        set_repeatable_read(&mut transaction).await?;
        let run = lock_run(&mut transaction, actor.operator_id, run_id).await?;
        if run.status != "approved" {
            return Err(RetentionError::InvalidState);
        }
        let data_class = RetentionDataClass::try_from(run.data_class.as_str())?;
        let current_counts = inventory_counts_transaction(
            &mut transaction,
            actor.operator_id,
            data_class,
            &run.provider,
            run.cutoff_at,
        )
        .await?;
        if current_counts != run.preview_counts {
            return Err(RetentionError::InventoryChanged);
        }
        let current_fingerprint = inventory_fingerprint_transaction(
            &mut transaction,
            actor.operator_id,
            data_class,
            &run.provider,
            run.cutoff_at,
        )
        .await?;
        if run.preview_fingerprint.as_deref() != Some(current_fingerprint.as_str()) {
            return Err(RetentionError::InventoryChanged);
        }
        let affected =
            execute_data_class(&mut transaction, actor.operator_id, &run, data_class, now).await?;
        let expected = inventory_total(&current_counts)? as u64;
        if affected != expected {
            return Err(RetentionError::InventoryChanged);
        }
        let deletion_counts = current_counts;
        sqlx::query(
            r#"
            UPDATE retention_runs
            SET status = 'completed', deletion_counts = $3,
                executed_by_identity_id = $4, started_at = $5, completed_at = $5
            WHERE operator_id = $1 AND id = $2
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(run.id)
        .bind(&deletion_counts)
        .bind(actor.identity_id)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        insert_completion_audit(
            &mut transaction,
            actor.operator_id,
            actor.identity_id,
            &run,
            &deletion_counts,
            now,
        )
        .await?;
        transaction.commit().await?;
        self.run(actor.operator_id, run_id).await
    }

    async fn policy(
        &self,
        operator_id: OperatorId,
        policy_id: Uuid,
    ) -> Result<RetentionPolicyView, RetentionError> {
        fetch_policy(&self.database, operator_id, policy_id)
            .await?
            .ok_or(RetentionError::NotFound)
    }

    async fn run(
        &self,
        operator_id: OperatorId,
        run_id: Uuid,
    ) -> Result<RetentionRunView, RetentionError> {
        fetch_run(&self.database, operator_id, run_id)
            .await?
            .ok_or(RetentionError::NotFound)
    }
}

pub fn retention_router(store: RetentionStore) -> Router {
    Router::new()
        .route(
            "/api/admin/retention/policies",
            get(list_policies).post(create_policy),
        )
        .route(
            "/api/admin/retention/policies/{policy_id}/approve",
            post(approve_policy),
        )
        .route("/api/admin/retention/runs/preview", post(preview_run))
        .route(
            "/api/admin/retention/runs/{run_id}/approve",
            post(approve_run),
        )
        .route(
            "/api/admin/retention/runs/{run_id}/execute",
            post(execute_run),
        )
        .merge(schedule::schedule_router())
        .merge(integrity::integrity_router())
        .with_state(store)
}

#[derive(Serialize)]
struct PolicyList {
    data: Vec<RetentionPolicyView>,
}

async fn list_policies(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<PolicyList>, RetentionHttpError> {
    authorize(&context)?;
    let data = store.list_policies(context.operator_id).await?;
    Ok(Json(PolicyList { data }))
}

async fn create_policy(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Json(request): Json<CreateRetentionPolicy>,
) -> Result<(StatusCode, Json<RetentionPolicyView>), RetentionHttpError> {
    authorize(&context)?;
    let policy = store.create_policy(&context, &request, Utc::now()).await?;
    Ok((StatusCode::CREATED, Json(policy)))
}

async fn approve_policy(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<RetentionPolicyView>, RetentionHttpError> {
    authorize(&context)?;
    Ok(Json(
        store
            .approve_policy(&context, policy_id, Utc::now())
            .await?,
    ))
}

async fn preview_run(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Json(request): Json<PreviewRetentionRun>,
) -> Result<(StatusCode, Json<RetentionRunView>), RetentionHttpError> {
    authorize(&context)?;
    let run = store.preview_run(&context, &request, Utc::now()).await?;
    Ok((StatusCode::CREATED, Json(run)))
}

async fn approve_run(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<RetentionRunView>, RetentionHttpError> {
    authorize(&context)?;
    Ok(Json(store.approve_run(&context, run_id, Utc::now()).await?))
}

async fn execute_run(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<RetentionRunView>, RetentionHttpError> {
    authorize(&context)?;
    Ok(Json(store.execute_run(&context, run_id, Utc::now()).await?))
}

#[derive(Debug, Error)]
enum RetentionHttpError {
    #[error("the current session is not authorized to manage retention")]
    Forbidden,
    #[error(transparent)]
    Retention(#[from] RetentionError),
}

impl IntoResponse for RetentionHttpError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "authorization_denied",
                self.to_string(),
            ),
            Self::Retention(RetentionError::InvalidConfiguration) => (
                StatusCode::BAD_REQUEST,
                "invalid_retention_configuration",
                self.to_string(),
            ),
            Self::Retention(RetentionError::NotFound) => (
                StatusCode::NOT_FOUND,
                "retention_record_not_found",
                self.to_string(),
            ),
            Self::Retention(RetentionError::SeparationOfDuties) => (
                StatusCode::CONFLICT,
                "second_administrator_required",
                self.to_string(),
            ),
            Self::Retention(RetentionError::InvalidState) => (
                StatusCode::CONFLICT,
                "invalid_retention_state",
                self.to_string(),
            ),
            Self::Retention(RetentionError::InventoryChanged) => (
                StatusCode::CONFLICT,
                "retention_inventory_changed",
                self.to_string(),
            ),
            Self::Retention(RetentionError::ScopeTooLarge) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "retention_scope_too_large",
                self.to_string(),
            ),
            Self::Retention(RetentionError::Database(_)) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "retention_unavailable",
                "Retention controls are temporarily unavailable".into(),
            ),
        };
        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}

fn authorize(context: &AuthContext) -> Result<(), RetentionHttpError> {
    require(context, Permission::ManageRetention).map_err(|_| RetentionHttpError::Forbidden)
}

fn bounded_name(value: &str) -> Result<String, RetentionError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RetentionError::InvalidConfiguration);
    }
    Ok(value.to_owned())
}

fn bounded_reference(value: &str) -> Result<String, RetentionError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 200
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'/' | b'.' | b'_' | b'-')
        })
    {
        return Err(RetentionError::InvalidConfiguration);
    }
    Ok(value.to_owned())
}

fn inventory_total(counts: &Value) -> Result<i64, RetentionError> {
    let object = counts.as_object().ok_or(RetentionError::InvalidState)?;
    object.values().try_fold(0_i64, |total, value| {
        value
            .as_i64()
            .and_then(|count| total.checked_add(count))
            .ok_or(RetentionError::InvalidState)
    })
}

async fn set_repeatable_read(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn inventory_counts_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    data_class: RetentionDataClass,
    provider: &str,
    cutoff_at: DateTime<Utc>,
) -> Result<Value, sqlx::Error> {
    if data_class == RetentionDataClass::NormalizedOperationalFact {
        let inventory =
            sqlx::query_as::<_, OperationalFactInventory>(OPERATIONAL_FACT_INVENTORY_SQL)
                .bind(operator_id.as_uuid())
                .bind(provider)
                .bind(cutoff_at)
                .fetch_one(&mut **transaction)
                .await?;
        return Ok(operational_fact_inventory_json(inventory));
    }
    if data_class == RetentionDataClass::TerminalAlertHistory {
        let inventory = sqlx::query_as::<_, AlertHistoryInventory>(ALERT_HISTORY_INVENTORY_SQL)
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_one(&mut **transaction)
            .await?;
        return Ok(json!({
            "alerts": inventory.alerts,
            "alert_actions": inventory.alert_actions,
            "alert_evidence": inventory.alert_evidence,
        }));
    }
    let count = match data_class {
        RetentionDataClass::ProviderRawPayload => {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM provider_envelopes
                WHERE operator_id = $1 AND provider = $2
                  AND received_at < $3 AND raw_payload_deleted_at IS NULL
                "#,
            )
            .bind(operator_id.as_uuid())
            .bind(provider)
            .bind(cutoff_at)
            .fetch_one(&mut **transaction)
            .await?
        }
        RetentionDataClass::AuthorizationAudit => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM authorization_audit_events WHERE operator_id = $1 AND occurred_at < $2",
            )
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_one(&mut **transaction)
            .await?
        }
        RetentionDataClass::SessionRevocation => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM auth_session_revocations WHERE operator_id = $1 AND expires_at < $2",
            )
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_one(&mut **transaction)
            .await?
        }
        RetentionDataClass::IdentityMapping => {
            sqlx::query_scalar::<_, i64>(identity_candidate_count_sql())
                .bind(operator_id.as_uuid())
                .bind(cutoff_at)
                .fetch_one(&mut **transaction)
                .await?
        }
        RetentionDataClass::TerminalAlertHistory => unreachable!(),
        RetentionDataClass::NormalizedOperationalFact => unreachable!(),
    };
    Ok(json!({ data_class.inventory_key(): count }))
}

fn operational_fact_inventory_json(inventory: OperationalFactInventory) -> Value {
    json!({
        "airport_observations": inventory.airport_observations,
        "flights": inventory.flights,
        "aircraft_positions": inventory.aircraft_positions,
        "planned_routes": inventory.planned_routes,
        "weather_hazards": inventory.weather_hazards,
    })
}

fn identity_candidate_count_sql() -> &'static str {
    r#"
    SELECT COUNT(*)
    FROM auth_identities identity
    WHERE identity.subject NOT LIKE 'deleted:%'
      AND EXISTS (
          SELECT 1 FROM operator_memberships own_membership
          WHERE own_membership.identity_id = identity.id
            AND own_membership.operator_id = $1
            AND own_membership.status = 'revoked'
            AND own_membership.revoked_at < $2
      )
      AND NOT EXISTS (
          SELECT 1 FROM operator_memberships any_membership
          WHERE any_membership.identity_id = identity.id
            AND (
                any_membership.operator_id <> $1
                OR any_membership.status <> 'revoked'
                OR any_membership.revoked_at IS NULL
                OR any_membership.revoked_at >= $2
            )
      )
    "#
}

async fn inventory_fingerprint_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    data_class: RetentionDataClass,
    provider: &str,
    cutoff_at: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    let mut keys = match data_class {
        RetentionDataClass::ProviderRawPayload => {
            sqlx::query_scalar::<_, String>(
                r#"
                SELECT 'provider_envelopes:' || id::text
                FROM provider_envelopes
                WHERE operator_id = $1 AND provider = $2
                  AND received_at < $3 AND raw_payload_deleted_at IS NULL
                "#,
            )
            .bind(operator_id.as_uuid())
            .bind(provider)
            .bind(cutoff_at)
            .fetch_all(&mut **transaction)
            .await?
        }
        RetentionDataClass::AuthorizationAudit => {
            sqlx::query_scalar::<_, String>(
                r#"
                SELECT 'authorization_audit_events:' || id::text
                FROM authorization_audit_events
                WHERE operator_id = $1 AND occurred_at < $2
                "#,
            )
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_all(&mut **transaction)
            .await?
        }
        RetentionDataClass::SessionRevocation => {
            sqlx::query_scalar::<_, String>(
                r#"
                SELECT 'auth_session_revocations:' || id::text
                FROM auth_session_revocations
                WHERE operator_id = $1 AND expires_at < $2
                "#,
            )
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_all(&mut **transaction)
            .await?
        }
        RetentionDataClass::IdentityMapping => {
            sqlx::query_scalar::<_, String>(
                r#"
                SELECT 'auth_identities:' || identity.id::text
                FROM auth_identities identity
                WHERE identity.subject NOT LIKE 'deleted:%'
                  AND EXISTS (
                      SELECT 1 FROM operator_memberships own_membership
                      WHERE own_membership.identity_id = identity.id
                        AND own_membership.operator_id = $1
                        AND own_membership.status = 'revoked'
                        AND own_membership.revoked_at < $2
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM operator_memberships any_membership
                      WHERE any_membership.identity_id = identity.id
                        AND (
                            any_membership.operator_id <> $1
                            OR any_membership.status <> 'revoked'
                            OR any_membership.revoked_at IS NULL
                            OR any_membership.revoked_at >= $2
                        )
                  )
                "#,
            )
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_all(&mut **transaction)
            .await?
        }
        RetentionDataClass::TerminalAlertHistory => {
            sqlx::query_scalar::<_, String>(
                r#"
                WITH eligible_series AS (
                    SELECT alert.series_key
                    FROM alerts alert
                    LEFT JOIN alert_actions action
                      ON action.operator_id = alert.operator_id AND action.alert_id = alert.id
                    WHERE alert.operator_id = $1
                    GROUP BY alert.series_key
                    HAVING BOOL_AND(alert.lifecycle IN ('dismissed', 'resolved'))
                       AND MAX(GREATEST(
                           alert.event_time,
                           alert.received_at,
                           alert.processed_at,
                           COALESCE(action.occurred_at, alert.processed_at)
                       )) < $2
                ), eligible_alerts AS (
                    SELECT alert.id
                    FROM alerts alert
                    JOIN eligible_series series ON series.series_key = alert.series_key
                    WHERE alert.operator_id = $1
                )
                SELECT inventory_key FROM (
                    SELECT 'alerts:' || alert.id::text AS inventory_key
                    FROM eligible_alerts alert
                    UNION ALL
                    SELECT 'alert_actions:' || action.id::text
                    FROM alert_actions action
                    JOIN eligible_alerts alert ON alert.id = action.alert_id
                    WHERE action.operator_id = $1
                    UNION ALL
                    SELECT 'alert_evidence:' || evidence.alert_id::text || ':' || evidence.source_envelope_id::text
                    FROM alert_evidence evidence
                    JOIN eligible_alerts alert ON alert.id = evidence.alert_id
                    WHERE evidence.operator_id = $1
                ) inventory
                "#,
            )
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_all(&mut **transaction)
            .await?
        }
        RetentionDataClass::NormalizedOperationalFact => {
            sqlx::query_scalar::<_, String>(
                r#"
                SELECT fact_type || ':' || record_id::text
                FROM eligible_normalized_fact_ids($1, $2, $3)
                "#,
            )
            .bind(operator_id.as_uuid())
            .bind(provider)
            .bind(cutoff_at)
            .fetch_all(&mut **transaction)
            .await?
        }
    };
    keys.sort_unstable();
    Ok(fingerprint_inventory(
        data_class, provider, cutoff_at, &keys,
    ))
}

fn fingerprint_inventory(
    data_class: RetentionDataClass,
    provider: &str,
    cutoff_at: DateTime<Utc>,
    keys: &[String],
) -> String {
    fn add_component(hasher: &mut Sha256, value: &[u8]) {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value);
    }

    let mut hasher = Sha256::new();
    add_component(&mut hasher, b"retention-inventory-v1");
    add_component(&mut hasher, data_class.as_str().as_bytes());
    add_component(&mut hasher, provider.as_bytes());
    add_component(&mut hasher, &cutoff_at.timestamp_micros().to_be_bytes());
    for key in keys {
        add_component(&mut hasher, key.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

async fn execute_data_class(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    run: &RetentionRunView,
    data_class: RetentionDataClass,
    now: DateTime<Utc>,
) -> Result<u64, RetentionError> {
    match data_class {
        RetentionDataClass::ProviderRawPayload => {
            execute_raw_payloads(transaction, operator_id, run, now).await
        }
        RetentionDataClass::AuthorizationAudit => {
            execute_lifecycle_deletion(
                transaction,
                operator_id,
                run,
                data_class,
                "authorization_audit_events",
                "occurred_at",
                now,
            )
            .await
        }
        RetentionDataClass::SessionRevocation => {
            execute_lifecycle_deletion(
                transaction,
                operator_id,
                run,
                data_class,
                "auth_session_revocations",
                "expires_at",
                now,
            )
            .await
        }
        RetentionDataClass::IdentityMapping => {
            execute_identity_minimization(transaction, operator_id, run, now).await
        }
        RetentionDataClass::TerminalAlertHistory => {
            execute_terminal_alert_history(transaction, operator_id, run, now).await
        }
        RetentionDataClass::NormalizedOperationalFact => {
            execute_normalized_operational_facts(transaction, operator_id, run, now).await
        }
    }
}

async fn execute_normalized_operational_facts(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    run: &RetentionRunView,
    now: DateTime<Utc>,
) -> Result<u64, RetentionError> {
    let facts = sqlx::query_as::<_, EligibleOperationalFact>(
        r#"
        SELECT fact_type, record_id
        FROM eligible_normalized_fact_ids($1, $2, $3)
        ORDER BY fact_type, record_id
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(&run.provider)
    .bind(run.cutoff_at)
    .fetch_all(&mut **transaction)
    .await?;

    for fact in &facts {
        sqlx::query(
            r#"
            INSERT INTO operational_fact_tombstones (
                id, operator_id, retention_run_id, provider,
                fact_type, record_id, deleted_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(operator_id.as_uuid())
        .bind(run.id)
        .bind(&run.provider)
        .bind(&fact.fact_type)
        .bind(fact.record_id)
        .bind(now)
        .execute(&mut **transaction)
        .await?;
    }

    let ids_for = |fact_type: &str| {
        facts
            .iter()
            .filter(|fact| fact.fact_type == fact_type)
            .map(|fact| fact.record_id)
            .collect::<Vec<_>>()
    };
    let positions = delete_operational_facts(
        transaction,
        operator_id,
        "aircraft_positions",
        &ids_for("aircraft_positions"),
    )
    .await?;
    let routes = delete_operational_facts(
        transaction,
        operator_id,
        "planned_routes",
        &ids_for("planned_routes"),
    )
    .await?;
    let observations = delete_operational_facts(
        transaction,
        operator_id,
        "airport_observations",
        &ids_for("airport_observations"),
    )
    .await?;
    let hazards = delete_operational_facts(
        transaction,
        operator_id,
        "weather_hazards",
        &ids_for("weather_hazards"),
    )
    .await?;
    let flights =
        delete_operational_facts(transaction, operator_id, "flights", &ids_for("flights")).await?;
    Ok(positions + routes + observations + hazards + flights)
}

async fn delete_operational_facts(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    table: &str,
    ids: &[Uuid],
) -> Result<u64, sqlx::Error> {
    let query = format!("DELETE FROM {table} WHERE operator_id = $1 AND id = ANY($2)");
    Ok(sqlx::query(&query)
        .bind(operator_id.as_uuid())
        .bind(ids)
        .execute(&mut **transaction)
        .await?
        .rows_affected())
}

async fn execute_terminal_alert_history(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    run: &RetentionRunView,
    now: DateTime<Utc>,
) -> Result<u64, RetentionError> {
    let alerts = sqlx::query_as::<_, EligibleAlert>(
        r#"
        WITH eligible_series AS (
            SELECT alert.series_key
            FROM alerts alert
            LEFT JOIN alert_actions action
              ON action.operator_id = alert.operator_id AND action.alert_id = alert.id
            WHERE alert.operator_id = $1
            GROUP BY alert.series_key
            HAVING BOOL_AND(alert.lifecycle IN ('dismissed', 'resolved'))
               AND MAX(GREATEST(
                   alert.event_time,
                   alert.received_at,
                   alert.processed_at,
                   COALESCE(action.occurred_at, alert.processed_at)
               )) < $2
        )
        SELECT alert.id, alert.dedupe_key, alert.series_key, alert.alert_revision
        FROM alerts alert
        JOIN eligible_series series ON series.series_key = alert.series_key
        WHERE alert.operator_id = $1
        ORDER BY alert.series_key, alert.alert_revision
        FOR UPDATE OF alert
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(run.cutoff_at)
    .fetch_all(&mut **transaction)
    .await?;

    for alert in &alerts {
        sqlx::query(
            r#"
            INSERT INTO alert_history_tombstones (
                id, operator_id, retention_run_id, alert_id, dedupe_key,
                series_key, alert_revision, deleted_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(operator_id.as_uuid())
        .bind(run.id)
        .bind(alert.id)
        .bind(&alert.dedupe_key)
        .bind(&alert.series_key)
        .bind(alert.alert_revision)
        .bind(now)
        .execute(&mut **transaction)
        .await?;
    }

    let alert_ids = alerts.iter().map(|alert| alert.id).collect::<Vec<_>>();
    let actions =
        sqlx::query("DELETE FROM alert_actions WHERE operator_id = $1 AND alert_id = ANY($2)")
            .bind(operator_id.as_uuid())
            .bind(&alert_ids)
            .execute(&mut **transaction)
            .await?
            .rows_affected();
    let evidence =
        sqlx::query("DELETE FROM alert_evidence WHERE operator_id = $1 AND alert_id = ANY($2)")
            .bind(operator_id.as_uuid())
            .bind(&alert_ids)
            .execute(&mut **transaction)
            .await?
            .rows_affected();
    let deleted_alerts = sqlx::query("DELETE FROM alerts WHERE operator_id = $1 AND id = ANY($2)")
        .bind(operator_id.as_uuid())
        .bind(&alert_ids)
        .execute(&mut **transaction)
        .await?
        .rows_affected();
    Ok(actions + evidence + deleted_alerts)
}

async fn execute_raw_payloads(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    run: &RetentionRunView,
    now: DateTime<Utc>,
) -> Result<u64, RetentionError> {
    let envelopes = sqlx::query_as::<_, EligibleEnvelope>(
        r#"
        SELECT id, feed, raw_payload_sha256
        FROM provider_envelopes
        WHERE operator_id = $1 AND provider = $2
          AND received_at < $3 AND raw_payload_deleted_at IS NULL
        ORDER BY received_at, id
        FOR UPDATE
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(&run.provider)
    .bind(run.cutoff_at)
    .fetch_all(&mut **transaction)
    .await?;
    for envelope in &envelopes {
        sqlx::query(
            r#"
            INSERT INTO data_deletion_tombstones (
                id, operator_id, retention_run_id, provider, feed,
                source_envelope_id, raw_payload_sha256, deleted_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(operator_id.as_uuid())
        .bind(run.id)
        .bind(&run.provider)
        .bind(&envelope.feed)
        .bind(envelope.id)
        .bind(&envelope.raw_payload_sha256)
        .bind(now)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(sqlx::query(
        r#"
        UPDATE provider_envelopes SET raw_payload = '{}'::jsonb
        WHERE operator_id = $1 AND provider = $2
          AND received_at < $3 AND raw_payload_deleted_at IS NULL
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(&run.provider)
    .bind(run.cutoff_at)
    .execute(&mut **transaction)
    .await?
    .rows_affected())
}

async fn execute_lifecycle_deletion(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    run: &RetentionRunView,
    data_class: RetentionDataClass,
    table: &str,
    time_column: &str,
    now: DateTime<Utc>,
) -> Result<u64, RetentionError> {
    let select = format!(
        "SELECT id FROM {table} WHERE operator_id = $1 AND {time_column} < $2 ORDER BY id FOR UPDATE"
    );
    let ids = sqlx::query_scalar::<_, Uuid>(&select)
        .bind(operator_id.as_uuid())
        .bind(run.cutoff_at)
        .fetch_all(&mut **transaction)
        .await?;
    insert_lifecycle_tombstones(transaction, operator_id, run.id, data_class, &ids, now).await?;
    let delete = format!("DELETE FROM {table} WHERE operator_id = $1 AND {time_column} < $2");
    Ok(sqlx::query(&delete)
        .bind(operator_id.as_uuid())
        .bind(run.cutoff_at)
        .execute(&mut **transaction)
        .await?
        .rows_affected())
}

async fn execute_identity_minimization(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    run: &RetentionRunView,
    now: DateTime<Utc>,
) -> Result<u64, RetentionError> {
    let ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT identity.id
        FROM auth_identities identity
        WHERE identity.subject NOT LIKE 'deleted:%'
          AND EXISTS (
              SELECT 1 FROM operator_memberships own_membership
              WHERE own_membership.identity_id = identity.id
                AND own_membership.operator_id = $1
                AND own_membership.status = 'revoked'
                AND own_membership.revoked_at < $2
          )
          AND NOT EXISTS (
              SELECT 1 FROM operator_memberships any_membership
              WHERE any_membership.identity_id = identity.id
                AND (
                    any_membership.operator_id <> $1
                    OR any_membership.status <> 'revoked'
                    OR any_membership.revoked_at IS NULL
                    OR any_membership.revoked_at >= $2
                )
          )
        ORDER BY identity.id
        FOR UPDATE OF identity
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(run.cutoff_at)
    .fetch_all(&mut **transaction)
    .await?;
    insert_lifecycle_tombstones(
        transaction,
        operator_id,
        run.id,
        RetentionDataClass::IdentityMapping,
        &ids,
        now,
    )
    .await?;
    Ok(sqlx::query(
        r#"
        UPDATE auth_identities SET subject = subject
        WHERE id = ANY($1) AND subject NOT LIKE 'deleted:%'
        "#,
    )
    .bind(&ids)
    .execute(&mut **transaction)
    .await?
    .rows_affected())
}

async fn insert_lifecycle_tombstones(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    run_id: Uuid,
    data_class: RetentionDataClass,
    ids: &[Uuid],
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    for record_id in ids {
        sqlx::query(
            r#"
            INSERT INTO lifecycle_deletion_tombstones (
                id, operator_id, retention_run_id, data_class, record_id, deleted_at
            ) VALUES ($1,$2,$3,$4,$5,$6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(operator_id.as_uuid())
        .bind(run_id)
        .bind(data_class.as_str())
        .bind(record_id)
        .bind(now)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

async fn fetch_policy(
    database: &PgPool,
    operator_id: OperatorId,
    policy_id: Uuid,
) -> Result<Option<RetentionPolicyView>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, operator_id, data_class, provider, version, retention_seconds,
               status, approval_reference, created_by_identity_id,
               approved_by_identity_id, created_at, approved_at, retired_at
        FROM retention_policies
        WHERE operator_id = $1 AND id = $2
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(policy_id)
    .fetch_optional(database)
    .await
}

async fn fetch_run(
    database: &PgPool,
    operator_id: OperatorId,
    run_id: Uuid,
) -> Result<Option<RetentionRunView>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, operator_id, policy_id, policy_version, data_class, provider,
               cutoff_at, status, preview_counts, preview_fingerprint, deletion_counts,
               requested_by_identity_id, approved_by_identity_id,
               executed_by_identity_id, evidence_reference, requested_at,
               approved_at, started_at, completed_at
        FROM retention_runs
        WHERE operator_id = $1 AND id = $2
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(run_id)
    .fetch_optional(database)
    .await
}

async fn lock_policy(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    policy_id: Uuid,
) -> Result<RetentionPolicyView, RetentionError> {
    sqlx::query_as(
        r#"
        SELECT id, operator_id, data_class, provider, version, retention_seconds,
               status, approval_reference, created_by_identity_id,
               approved_by_identity_id, created_at, approved_at, retired_at
        FROM retention_policies
        WHERE operator_id = $1 AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(policy_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(RetentionError::NotFound)
}

async fn lock_run(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    run_id: Uuid,
) -> Result<RetentionRunView, RetentionError> {
    sqlx::query_as(
        r#"
        SELECT id, operator_id, policy_id, policy_version, data_class, provider,
               cutoff_at, status, preview_counts, preview_fingerprint, deletion_counts,
               requested_by_identity_id, approved_by_identity_id,
               executed_by_identity_id, evidence_reference, requested_at,
               approved_at, started_at, completed_at
        FROM retention_runs
        WHERE operator_id = $1 AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(run_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(RetentionError::NotFound)
}

async fn insert_completion_audit(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    actor_identity_id: Uuid,
    run: &RetentionRunView,
    deletion_counts: &Value,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO authorization_audit_events (
            id, operator_id, actor_identity_id, action, target_type,
            target_id, occurred_at, metadata
        ) VALUES ($1,$2,$3,'retention.run.completed','retention_run',$4,$5,$6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(operator_id.as_uuid())
    .bind(actor_identity_id)
    .bind(run.id.to_string())
    .bind(now)
    .bind(json!({
        "policy_id": run.policy_id,
        "policy_version": run.policy_version,
        "data_class": run.data_class,
        "provider": run.provider,
        "cutoff_at": run.cutoff_at,
        "counts": deletion_counts,
        "preview_fingerprint": run.preview_fingerprint,
        "requested_by_identity_id": run.requested_by_identity_id,
        "approved_by_identity_id": run.approved_by_identity_id,
        "evidence_reference": run.evidence_reference,
    }))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_and_evidence_references_are_bounded_and_non_free_form() {
        assert_eq!(bounded_name(" noaa-awc ").unwrap(), "noaa-awc");
        assert!(bounded_name("provider with spaces").is_err());
        assert_eq!(
            bounded_reference("incident:FT-401/run_1").unwrap(),
            "incident:FT-401/run_1"
        );
        assert!(bounded_reference("contains a secret note").is_err());
        assert_eq!(
            RetentionDataClass::try_from("authorization_audit").unwrap(),
            RetentionDataClass::AuthorizationAudit
        );
        assert_eq!(
            RetentionDataClass::try_from("terminal_alert_history").unwrap(),
            RetentionDataClass::TerminalAlertHistory
        );
        assert_eq!(
            RetentionDataClass::try_from("normalized_operational_fact").unwrap(),
            RetentionDataClass::NormalizedOperationalFact
        );
        assert!(RetentionDataClass::try_from("passenger_records").is_err());
    }

    #[test]
    fn inventory_fingerprint_binds_scope_cutoff_and_exact_record_keys() {
        let cutoff = DateTime::from_timestamp_micros(1_750_000_000_123_456).unwrap();
        let first = vec!["provider_envelopes:00000000-0000-0000-0000-000000000001".into()];
        let replacement = vec!["provider_envelopes:00000000-0000-0000-0000-000000000002".into()];

        let fingerprint = fingerprint_inventory(
            RetentionDataClass::ProviderRawPayload,
            "provider-a",
            cutoff,
            &first,
        );
        assert_eq!(fingerprint.len(), 64);
        assert_ne!(
            fingerprint,
            fingerprint_inventory(
                RetentionDataClass::ProviderRawPayload,
                "provider-a",
                cutoff,
                &replacement,
            )
        );
        assert_ne!(
            fingerprint,
            fingerprint_inventory(
                RetentionDataClass::ProviderRawPayload,
                "provider-b",
                cutoff,
                &first,
            )
        );
        assert_ne!(
            fingerprint,
            fingerprint_inventory(
                RetentionDataClass::AuthorizationAudit,
                "provider-a",
                cutoff,
                &first,
            )
        );
    }
}
