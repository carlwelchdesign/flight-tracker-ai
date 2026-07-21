use std::time::Duration as StdDuration;

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{auth::AuthContext, domain::OperatorId, health::WorkerProbe};

use super::{
    MAX_DELETE_RECORDS, RetentionDataClass, RetentionError, RetentionHttpError,
    RetentionPolicyView, RetentionRunView, RetentionStore, authorize, bounded_reference,
    execute_data_class, insert_completion_audit, inventory_counts_transaction,
    inventory_fingerprint_transaction, inventory_total, lock_policy, set_repeatable_read,
};

const MIN_CADENCE_SECONDS: i64 = 3_600;
const MAX_CADENCE_SECONDS: i64 = 2_678_400;
const MAX_FIRST_RUN_DELAY_SECONDS: i64 = 31_622_400;
const DEFAULT_DUE_LIMIT: i64 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateRetentionSchedule {
    pub policy_id: Uuid,
    pub cadence_seconds: i64,
    pub first_run_at: DateTime<Utc>,
    pub approval_reference: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct RetentionScheduleView {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub policy_id: Uuid,
    pub policy_version: i32,
    pub cadence_seconds: i64,
    pub status: String,
    pub approval_reference: String,
    pub created_by_identity_id: Uuid,
    pub approved_by_identity_id: Option<Uuid>,
    pub next_run_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_completed_at: Option<DateTime<Utc>>,
    pub consecutive_failures: i32,
    pub last_error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub paused_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct RetentionScheduleAttemptView {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub schedule_id: Uuid,
    pub scheduled_for: DateTime<Utc>,
    pub retention_run_id: Option<Uuid>,
    pub status: String,
    pub error_code: Option<String>,
    pub preview_counts: Value,
    pub preview_fingerprint: Option<String>,
    pub attempted_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

impl RetentionStore {
    pub async fn list_schedules(
        &self,
        operator_id: OperatorId,
    ) -> Result<Vec<RetentionScheduleView>, RetentionError> {
        Ok(sqlx::query_as::<_, RetentionScheduleView>(
            r#"
            SELECT id, operator_id, policy_id, policy_version, cadence_seconds,
                   status, approval_reference, created_by_identity_id,
                   approved_by_identity_id, next_run_at, last_attempt_at,
                   last_completed_at, consecutive_failures, last_error_code,
                   created_at, approved_at, paused_at
            FROM retention_schedules
            WHERE operator_id = $1
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(operator_id.as_uuid())
        .fetch_all(&self.database)
        .await?)
    }

    pub async fn create_schedule(
        &self,
        actor: &AuthContext,
        request: &CreateRetentionSchedule,
        now: DateTime<Utc>,
    ) -> Result<RetentionScheduleView, RetentionError> {
        if !(MIN_CADENCE_SECONDS..=MAX_CADENCE_SECONDS).contains(&request.cadence_seconds)
            || request.first_run_at > now + Duration::seconds(MAX_FIRST_RUN_DELAY_SECONDS)
        {
            return Err(RetentionError::InvalidConfiguration);
        }
        let approval_reference = bounded_reference(&request.approval_reference)?;
        let policy = self.policy(actor.operator_id, request.policy_id).await?;
        if policy.status != "approved" {
            return Err(RetentionError::InvalidState);
        }
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO retention_schedules (
                id, operator_id, policy_id, policy_version, cadence_seconds,
                status, approval_reference, created_by_identity_id,
                next_run_at, created_at
            ) VALUES ($1,$2,$3,$4,$5,'draft',$6,$7,$8,$9)
            "#,
        )
        .bind(id)
        .bind(actor.operator_id.as_uuid())
        .bind(policy.id)
        .bind(policy.version)
        .bind(request.cadence_seconds)
        .bind(approval_reference)
        .bind(actor.identity_id)
        .bind(request.first_run_at.max(now))
        .bind(now)
        .execute(&self.database)
        .await?;
        self.schedule(actor.operator_id, id).await
    }

    pub async fn approve_schedule(
        &self,
        actor: &AuthContext,
        schedule_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RetentionScheduleView, RetentionError> {
        let mut transaction = self.database.begin().await?;
        let schedule = lock_schedule(&mut transaction, actor.operator_id, schedule_id).await?;
        if schedule.status != "draft" {
            return Err(RetentionError::InvalidState);
        }
        if schedule.created_by_identity_id == actor.identity_id {
            return Err(RetentionError::SeparationOfDuties);
        }
        let policy = lock_policy(&mut transaction, actor.operator_id, schedule.policy_id).await?;
        if policy.status != "approved" || policy.version != schedule.policy_version {
            return Err(RetentionError::InvalidState);
        }
        sqlx::query(
            r#"
            UPDATE retention_schedules
            SET status = 'retired', paused_at = $3
            WHERE operator_id = $1 AND policy_id = $2 AND status = 'active'
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(schedule.policy_id)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE retention_schedules
            SET status = 'active', approved_by_identity_id = $3,
                approved_at = $4, next_run_at = GREATEST(next_run_at, $4)
            WHERE operator_id = $1 AND id = $2 AND status = 'draft'
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(schedule_id)
        .bind(actor.identity_id)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.schedule(actor.operator_id, schedule_id).await
    }

    pub async fn pause_schedule(
        &self,
        actor: &AuthContext,
        schedule_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RetentionScheduleView, RetentionError> {
        let updated = sqlx::query(
            r#"
            UPDATE retention_schedules
            SET status = 'paused', paused_at = $3
            WHERE operator_id = $1 AND id = $2 AND status = 'active'
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(schedule_id)
        .bind(now)
        .execute(&self.database)
        .await?;
        if updated.rows_affected() != 1 {
            return match self.schedule(actor.operator_id, schedule_id).await {
                Ok(_) => Err(RetentionError::InvalidState),
                Err(error) => Err(error),
            };
        }
        self.schedule(actor.operator_id, schedule_id).await
    }

    pub async fn list_schedule_attempts(
        &self,
        operator_id: OperatorId,
        schedule_id: Uuid,
    ) -> Result<Vec<RetentionScheduleAttemptView>, RetentionError> {
        self.schedule(operator_id, schedule_id).await?;
        Ok(sqlx::query_as::<_, RetentionScheduleAttemptView>(
            r#"
            SELECT id, operator_id, schedule_id, scheduled_for,
                   retention_run_id, status, error_code, preview_counts,
                   preview_fingerprint,
                   attempted_at, completed_at
            FROM retention_schedule_attempts
            WHERE operator_id = $1 AND schedule_id = $2
            ORDER BY scheduled_for DESC, id DESC
            LIMIT 200
            "#,
        )
        .bind(operator_id.as_uuid())
        .bind(schedule_id)
        .fetch_all(&self.database)
        .await?)
    }

    pub async fn run_due_schedules(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<usize, RetentionError> {
        if !(1..=100).contains(&limit) {
            return Err(RetentionError::InvalidConfiguration);
        }
        let ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id FROM retention_schedules
            WHERE status = 'active' AND next_run_at <= $1
            ORDER BY next_run_at, operator_id, id
            LIMIT $2
            "#,
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&self.database)
        .await?;
        let mut processed = 0;
        for schedule_id in ids {
            if self.run_due_schedule(schedule_id, now).await? {
                processed += 1;
            }
        }
        Ok(processed)
    }

    async fn run_due_schedule(
        &self,
        schedule_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, RetentionError> {
        let mut transaction = self.database.begin().await?;
        set_repeatable_read(&mut transaction).await?;
        let Some(schedule) = lock_due_schedule(&mut transaction, schedule_id, now).await? else {
            return Ok(false);
        };
        let policy = lock_policy(
            &mut transaction,
            OperatorId::from_uuid(schedule.operator_id),
            schedule.policy_id,
        )
        .await?;
        let next_run_at = next_scheduled_time(schedule.next_run_at, schedule.cadence_seconds, now)?;
        if policy.status != "approved" || policy.version != schedule.policy_version {
            record_schedule_failure(
                &mut transaction,
                &schedule,
                "policy_not_approved",
                &json!({}),
                None,
                next_run_at,
                now,
            )
            .await?;
            transaction.commit().await?;
            return Ok(true);
        }
        if !schedule_actors_are_active(&mut transaction, &schedule).await? {
            record_schedule_failure(
                &mut transaction,
                &schedule,
                "schedule_authorization_inactive",
                &json!({}),
                None,
                next_run_at,
                now,
            )
            .await?;
            transaction.commit().await?;
            return Ok(true);
        }

        let operator_id = OperatorId::from_uuid(schedule.operator_id);
        let data_class = RetentionDataClass::try_from(policy.data_class.as_str())?;
        let cutoff_at = now - Duration::seconds(policy.retention_seconds);
        let preview_counts = inventory_counts_transaction(
            &mut transaction,
            operator_id,
            data_class,
            &policy.provider,
            cutoff_at,
        )
        .await?;
        if inventory_total(&preview_counts)? > MAX_DELETE_RECORDS {
            record_schedule_failure(
                &mut transaction,
                &schedule,
                "retention_scope_too_large",
                &preview_counts,
                None,
                next_run_at,
                now,
            )
            .await?;
            transaction.commit().await?;
            return Ok(true);
        }
        let preview_fingerprint = inventory_fingerprint_transaction(
            &mut transaction,
            operator_id,
            data_class,
            &policy.provider,
            cutoff_at,
        )
        .await?;

        sqlx::query("SAVEPOINT scheduled_retention_execution")
            .execute(&mut *transaction)
            .await?;
        let run = insert_scheduled_run(
            &mut transaction,
            &schedule,
            &policy,
            cutoff_at,
            &preview_counts,
            &preview_fingerprint,
            now,
        )
        .await?;
        let affected =
            execute_data_class(&mut transaction, operator_id, &run, data_class, now).await?;
        let expected = inventory_total(&preview_counts)? as u64;
        if affected != expected {
            sqlx::query("ROLLBACK TO SAVEPOINT scheduled_retention_execution")
                .execute(&mut *transaction)
                .await?;
            record_schedule_failure(
                &mut transaction,
                &schedule,
                "retention_inventory_changed",
                &preview_counts,
                Some(&preview_fingerprint),
                next_run_at,
                now,
            )
            .await?;
            transaction.commit().await?;
            return Ok(true);
        }
        sqlx::query("RELEASE SAVEPOINT scheduled_retention_execution")
            .execute(&mut *transaction)
            .await?;

        sqlx::query(
            r#"
            UPDATE retention_runs
            SET status = 'completed', deletion_counts = $3,
                executed_by_identity_id = $4, started_at = $5, completed_at = $5
            WHERE operator_id = $1 AND id = $2 AND status = 'approved'
            "#,
        )
        .bind(schedule.operator_id)
        .bind(run.id)
        .bind(&preview_counts)
        .bind(schedule.created_by_identity_id)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        insert_completion_audit(
            &mut transaction,
            operator_id,
            schedule.created_by_identity_id,
            &run,
            &preview_counts,
            now,
        )
        .await?;
        sqlx::query(
            r#"
            INSERT INTO retention_schedule_attempts (
                id, operator_id, schedule_id, scheduled_for, retention_run_id,
                status, preview_counts, preview_fingerprint, attempted_at, completed_at
            ) VALUES ($1,$2,$3,$4,$5,'completed',$6,$7,$8,$8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(schedule.operator_id)
        .bind(schedule.id)
        .bind(schedule.next_run_at)
        .bind(run.id)
        .bind(&preview_counts)
        .bind(&preview_fingerprint)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE retention_schedules
            SET next_run_at = $3, last_attempt_at = $4, last_completed_at = $4,
                consecutive_failures = 0, last_error_code = NULL
            WHERE operator_id = $1 AND id = $2 AND status = 'active'
            "#,
        )
        .bind(schedule.operator_id)
        .bind(schedule.id)
        .bind(next_run_at)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(true)
    }

    async fn schedule(
        &self,
        operator_id: OperatorId,
        schedule_id: Uuid,
    ) -> Result<RetentionScheduleView, RetentionError> {
        fetch_schedule(&self.database, operator_id, schedule_id)
            .await?
            .ok_or(RetentionError::NotFound)
    }
}

async fn insert_scheduled_run(
    transaction: &mut Transaction<'_, Postgres>,
    schedule: &RetentionScheduleView,
    policy: &RetentionPolicyView,
    cutoff_at: DateTime<Utc>,
    preview_counts: &Value,
    preview_fingerprint: &str,
    now: DateTime<Utc>,
) -> Result<RetentionRunView, sqlx::Error> {
    let id = Uuid::new_v4();
    let evidence_reference = format!(
        "schedule:{}:{}",
        schedule.id,
        schedule.next_run_at.timestamp()
    );
    sqlx::query(
        r#"
        INSERT INTO retention_runs (
            id, operator_id, policy_id, policy_version, data_class, provider,
            cutoff_at, status, preview_counts, preview_fingerprint,
            requested_by_identity_id, approved_by_identity_id,
            evidence_reference, requested_at, approved_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,'approved',$8,$9,$10,$11,$12,$13,$13)
        "#,
    )
    .bind(id)
    .bind(schedule.operator_id)
    .bind(policy.id)
    .bind(policy.version)
    .bind(&policy.data_class)
    .bind(&policy.provider)
    .bind(cutoff_at)
    .bind(preview_counts)
    .bind(preview_fingerprint)
    .bind(schedule.created_by_identity_id)
    .bind(schedule.approved_by_identity_id)
    .bind(&evidence_reference)
    .bind(now)
    .execute(&mut **transaction)
    .await?;
    Ok(RetentionRunView {
        id,
        operator_id: schedule.operator_id,
        policy_id: policy.id,
        policy_version: policy.version,
        data_class: policy.data_class.clone(),
        provider: policy.provider.clone(),
        cutoff_at,
        status: "approved".into(),
        preview_counts: preview_counts.clone(),
        preview_fingerprint: Some(preview_fingerprint.into()),
        deletion_counts: None,
        requested_by_identity_id: schedule.created_by_identity_id,
        approved_by_identity_id: schedule.approved_by_identity_id,
        executed_by_identity_id: None,
        evidence_reference,
        requested_at: now,
        approved_at: Some(now),
        started_at: None,
        completed_at: None,
    })
}

async fn record_schedule_failure(
    transaction: &mut Transaction<'_, Postgres>,
    schedule: &RetentionScheduleView,
    error_code: &str,
    preview_counts: &Value,
    preview_fingerprint: Option<&str>,
    next_run_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO retention_schedule_attempts (
            id, operator_id, schedule_id, scheduled_for, status, error_code,
            preview_counts, preview_fingerprint, attempted_at, completed_at
        ) VALUES ($1,$2,$3,$4,'failed',$5,$6,$7,$8,$8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(schedule.operator_id)
    .bind(schedule.id)
    .bind(schedule.next_run_at)
    .bind(error_code)
    .bind(preview_counts)
    .bind(preview_fingerprint)
    .bind(now)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        r#"
        UPDATE retention_schedules
        SET status = 'paused', paused_at = $3, next_run_at = $4,
            last_attempt_at = $3, consecutive_failures = consecutive_failures + 1,
            last_error_code = $5
        WHERE operator_id = $1 AND id = $2 AND status = 'active'
        "#,
    )
    .bind(schedule.operator_id)
    .bind(schedule.id)
    .bind(now)
    .bind(next_run_at)
    .bind(error_code)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn schedule_actors_are_active(
    transaction: &mut Transaction<'_, Postgres>,
    schedule: &RetentionScheduleView,
) -> Result<bool, sqlx::Error> {
    let Some(approved_by_identity_id) = schedule.approved_by_identity_id else {
        return Ok(false);
    };
    let active = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(DISTINCT membership.identity_id)
        FROM operator_memberships membership
        JOIN auth_identities identity ON identity.id = membership.identity_id
        WHERE membership.operator_id = $1
          AND membership.identity_id = ANY($2)
          AND membership.status = 'active'
          AND membership.role = 'administrator'
          AND identity.disabled_at IS NULL
        "#,
    )
    .bind(schedule.operator_id)
    .bind(vec![
        schedule.created_by_identity_id,
        approved_by_identity_id,
    ])
    .fetch_one(&mut **transaction)
    .await?;
    Ok(active == 2)
}

fn next_scheduled_time(
    scheduled_for: DateTime<Utc>,
    cadence_seconds: i64,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, RetentionError> {
    if !(MIN_CADENCE_SECONDS..=MAX_CADENCE_SECONDS).contains(&cadence_seconds) {
        return Err(RetentionError::InvalidConfiguration);
    }
    let seconds_behind = now
        .signed_duration_since(scheduled_for)
        .num_seconds()
        .max(0);
    let steps = seconds_behind
        .checked_div(cadence_seconds)
        .and_then(|steps| steps.checked_add(1))
        .ok_or(RetentionError::InvalidConfiguration)?;
    let offset = cadence_seconds
        .checked_mul(steps)
        .ok_or(RetentionError::InvalidConfiguration)?;
    scheduled_for
        .checked_add_signed(Duration::seconds(offset))
        .ok_or(RetentionError::InvalidConfiguration)
}

async fn fetch_schedule(
    database: &sqlx::PgPool,
    operator_id: OperatorId,
    schedule_id: Uuid,
) -> Result<Option<RetentionScheduleView>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, operator_id, policy_id, policy_version, cadence_seconds,
               status, approval_reference, created_by_identity_id,
               approved_by_identity_id, next_run_at, last_attempt_at,
               last_completed_at, consecutive_failures, last_error_code,
               created_at, approved_at, paused_at
        FROM retention_schedules
        WHERE operator_id = $1 AND id = $2
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(schedule_id)
    .fetch_optional(database)
    .await
}

async fn lock_schedule(
    transaction: &mut Transaction<'_, Postgres>,
    operator_id: OperatorId,
    schedule_id: Uuid,
) -> Result<RetentionScheduleView, RetentionError> {
    sqlx::query_as(
        r#"
        SELECT id, operator_id, policy_id, policy_version, cadence_seconds,
               status, approval_reference, created_by_identity_id,
               approved_by_identity_id, next_run_at, last_attempt_at,
               last_completed_at, consecutive_failures, last_error_code,
               created_at, approved_at, paused_at
        FROM retention_schedules
        WHERE operator_id = $1 AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(schedule_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(RetentionError::NotFound)
}

async fn lock_due_schedule(
    transaction: &mut Transaction<'_, Postgres>,
    schedule_id: Uuid,
    now: DateTime<Utc>,
) -> Result<Option<RetentionScheduleView>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, operator_id, policy_id, policy_version, cadence_seconds,
               status, approval_reference, created_by_identity_id,
               approved_by_identity_id, next_run_at, last_attempt_at,
               last_completed_at, consecutive_failures, last_error_code,
               created_at, approved_at, paused_at
        FROM retention_schedules
        WHERE id = $1 AND status = 'active' AND next_run_at <= $2
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(schedule_id)
    .bind(now)
    .fetch_optional(&mut **transaction)
    .await
}

pub fn spawn_retention_scheduler(
    store: RetentionStore,
    scan_interval: StdDuration,
    mut probe: WorkerProbe,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut heartbeat = tokio::time::interval(StdDuration::from_secs(1));
        let mut scan = tokio::time::interval(scan_interval);
        probe.heartbeat();
        loop {
            tokio::select! {
                _ = heartbeat.tick() => probe.heartbeat(),
                _ = scan.tick() => {
                    match store.run_due_schedules(Utc::now(), DEFAULT_DUE_LIMIT).await {
                        Ok(processed) => {
                            if processed > 0 {
                                tracing::info!(worker = "retention_scheduler", processed, "processed due retention schedules");
                            }
                            probe.heartbeat();
                        }
                        Err(error) => {
                            tracing::error!(worker = "retention_scheduler", error = %error, "retention schedule scan failed");
                            probe.fail("retention schedule scan failed");
                            break;
                        }
                    }
                }
            }
        }
    })
}

pub(super) fn schedule_router() -> Router<RetentionStore> {
    Router::new()
        .route(
            "/api/admin/retention/schedules",
            get(list_schedules).post(create_schedule),
        )
        .route(
            "/api/admin/retention/schedules/{schedule_id}/approve",
            post(approve_schedule),
        )
        .route(
            "/api/admin/retention/schedules/{schedule_id}/pause",
            post(pause_schedule),
        )
        .route(
            "/api/admin/retention/schedules/{schedule_id}/attempts",
            get(list_schedule_attempts),
        )
}

#[derive(Serialize)]
struct ScheduleList {
    data: Vec<RetentionScheduleView>,
}

#[derive(Serialize)]
struct ScheduleAttemptList {
    data: Vec<RetentionScheduleAttemptView>,
}

async fn list_schedules(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<ScheduleList>, RetentionHttpError> {
    authorize(&context)?;
    Ok(Json(ScheduleList {
        data: store.list_schedules(context.operator_id).await?,
    }))
}

async fn create_schedule(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Json(request): Json<CreateRetentionSchedule>,
) -> Result<(StatusCode, Json<RetentionScheduleView>), RetentionHttpError> {
    authorize(&context)?;
    let schedule = store
        .create_schedule(&context, &request, Utc::now())
        .await?;
    Ok((StatusCode::CREATED, Json(schedule)))
}

async fn approve_schedule(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Path(schedule_id): Path<Uuid>,
) -> Result<Json<RetentionScheduleView>, RetentionHttpError> {
    authorize(&context)?;
    Ok(Json(
        store
            .approve_schedule(&context, schedule_id, Utc::now())
            .await?,
    ))
}

async fn pause_schedule(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Path(schedule_id): Path<Uuid>,
) -> Result<Json<RetentionScheduleView>, RetentionHttpError> {
    authorize(&context)?;
    Ok(Json(
        store
            .pause_schedule(&context, schedule_id, Utc::now())
            .await?,
    ))
}

async fn list_schedule_attempts(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
    Path(schedule_id): Path<Uuid>,
) -> Result<Json<ScheduleAttemptList>, RetentionHttpError> {
    authorize(&context)?;
    Ok(Json(ScheduleAttemptList {
        data: store
            .list_schedule_attempts(context.operator_id, schedule_id)
            .await?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadence_advances_from_the_original_slot_without_drift() {
        let scheduled_for = DateTime::parse_from_rfc3339("2026-07-21T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            next_scheduled_time(
                scheduled_for,
                3_600,
                scheduled_for + Duration::seconds(7_201)
            )
            .unwrap(),
            scheduled_for + Duration::seconds(10_800)
        );
        assert!(next_scheduled_time(scheduled_for, 60, scheduled_for).is_err());
    }
}
