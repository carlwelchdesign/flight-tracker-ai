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
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, Permission, require},
    domain::OperatorId,
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
}

impl RetentionDataClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProviderRawPayload => "provider_raw_payload",
            Self::AuthorizationAudit => "authorization_audit",
            Self::SessionRevocation => "session_revocation",
            Self::IdentityMapping => "identity_mapping",
        }
    }

    const fn inventory_key(self) -> &'static str {
        match self {
            Self::ProviderRawPayload => "provider_envelopes",
            Self::AuthorizationAudit => "authorization_audit_events",
            Self::SessionRevocation => "auth_session_revocations",
            Self::IdentityMapping => "auth_identities_minimized",
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
        if request.data_class != RetentionDataClass::ProviderRawPayload && provider != "application"
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
        let policy = self.policy(actor.operator_id, request.policy_id).await?;
        if policy.status != "approved" {
            return Err(RetentionError::InvalidState);
        }
        let cutoff_at = now - Duration::seconds(policy.retention_seconds);
        let data_class = RetentionDataClass::try_from(policy.data_class.as_str())?;
        let preview_counts = inventory_counts(
            &self.database,
            actor.operator_id,
            data_class,
            &policy.provider,
            cutoff_at,
        )
        .await?;
        if inventory_total(&preview_counts)? > MAX_DELETE_RECORDS {
            return Err(RetentionError::ScopeTooLarge);
        }
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO retention_runs (
                id, operator_id, policy_id, policy_version, data_class, provider,
                cutoff_at, status, preview_counts, requested_by_identity_id,
                evidence_reference, requested_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,'awaiting_approval',$8,$9,$10,$11)
            "#,
        )
        .bind(id)
        .bind(actor.operator_id.as_uuid())
        .bind(policy.id)
        .bind(policy.version)
        .bind(&policy.data_class)
        .bind(&policy.provider)
        .bind(cutoff_at)
        .bind(preview_counts)
        .bind(actor.identity_id)
        .bind(evidence_reference)
        .bind(now)
        .execute(&self.database)
        .await?;
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
        insert_completion_audit(&mut transaction, actor, &run, &deletion_counts, now).await?;
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

async fn inventory_counts(
    database: &PgPool,
    operator_id: OperatorId,
    data_class: RetentionDataClass,
    provider: &str,
    cutoff_at: DateTime<Utc>,
) -> Result<Value, sqlx::Error> {
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
            .fetch_one(database)
            .await?
        }
        RetentionDataClass::AuthorizationAudit => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM authorization_audit_events WHERE operator_id = $1 AND occurred_at < $2",
            )
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_one(database)
            .await?
        }
        RetentionDataClass::SessionRevocation => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM auth_session_revocations WHERE operator_id = $1 AND expires_at < $2",
            )
            .bind(operator_id.as_uuid())
            .bind(cutoff_at)
            .fetch_one(database)
            .await?
        }
        RetentionDataClass::IdentityMapping => {
            sqlx::query_scalar::<_, i64>(identity_candidate_count_sql())
                .bind(operator_id.as_uuid())
                .bind(cutoff_at)
                .fetch_one(database)
                .await?
        }
    };
    Ok(json!({ data_class.inventory_key(): count }))
}

async fn inventory_counts_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    data_class: RetentionDataClass,
    provider: &str,
    cutoff_at: DateTime<Utc>,
) -> Result<Value, sqlx::Error> {
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
    };
    Ok(json!({ data_class.inventory_key(): count }))
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
    }
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
               cutoff_at, status, preview_counts, deletion_counts,
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
               cutoff_at, status, preview_counts, deletion_counts,
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
    actor: &AuthContext,
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
    .bind(actor.operator_id.as_uuid())
    .bind(actor.identity_id)
    .bind(run.id.to_string())
    .bind(now)
    .bind(json!({
        "policy_id": run.policy_id,
        "policy_version": run.policy_version,
        "data_class": run.data_class,
        "provider": run.provider,
        "cutoff_at": run.cutoff_at,
        "counts": deletion_counts,
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
        assert!(RetentionDataClass::try_from("passenger_records").is_err());
    }
}
