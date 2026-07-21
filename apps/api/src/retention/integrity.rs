use axum::{
    Json, Router,
    extract::{Extension, State},
    routing::get,
};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::AuthContext, domain::OperatorId};

use super::{RetentionError, RetentionHttpError, RetentionStore, authorize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetentionIntegrityView {
    pub operator_id: Uuid,
    pub evaluated_at: DateTime<Utc>,
    pub healthy: bool,
    pub violations: RetentionIntegrityViolations,
    pub tombstones: RetentionTombstoneCounts,
    pub paused_schedules: i64,
    pub failed_attempts_24h: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetentionIntegrityViolations {
    pub raw_payloads: i64,
    pub authorization_audit: i64,
    pub session_revocations: i64,
    pub identity_mappings: i64,
    pub alert_history: i64,
    pub operational_facts: i64,
}

impl RetentionIntegrityViolations {
    fn total(&self) -> i64 {
        self.raw_payloads
            + self.authorization_audit
            + self.session_revocations
            + self.identity_mappings
            + self.alert_history
            + self.operational_facts
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetentionTombstoneCounts {
    pub raw_payloads: i64,
    pub lifecycle: i64,
    pub alert_history: i64,
    pub operational_facts: i64,
}

#[derive(FromRow)]
struct IntegrityRow {
    raw_payload_violations: i64,
    authorization_audit_violations: i64,
    session_revocation_violations: i64,
    identity_mapping_violations: i64,
    alert_history_violations: i64,
    operational_fact_violations: i64,
    raw_payload_tombstones: i64,
    lifecycle_tombstones: i64,
    alert_history_tombstones: i64,
    operational_fact_tombstones: i64,
    paused_schedules: i64,
    failed_attempts_24h: i64,
}

impl RetentionStore {
    pub async fn retention_integrity(
        &self,
        operator_id: OperatorId,
        evaluated_at: DateTime<Utc>,
    ) -> Result<RetentionIntegrityView, RetentionError> {
        let row = sqlx::query_as::<_, IntegrityRow>(
            r#"
            WITH operational_fact_violations AS (
                SELECT tombstone.record_id
                FROM operational_fact_tombstones tombstone
                JOIN airport_observations fact
                  ON tombstone.fact_type = 'airport_observations'
                 AND fact.operator_id = tombstone.operator_id
                 AND fact.id = tombstone.record_id
                WHERE tombstone.operator_id = $1
                UNION ALL
                SELECT tombstone.record_id
                FROM operational_fact_tombstones tombstone
                JOIN flights fact
                  ON tombstone.fact_type = 'flights'
                 AND fact.operator_id = tombstone.operator_id
                 AND fact.id = tombstone.record_id
                WHERE tombstone.operator_id = $1
                UNION ALL
                SELECT tombstone.record_id
                FROM operational_fact_tombstones tombstone
                JOIN aircraft_positions fact
                  ON tombstone.fact_type = 'aircraft_positions'
                 AND fact.operator_id = tombstone.operator_id
                 AND fact.id = tombstone.record_id
                WHERE tombstone.operator_id = $1
                UNION ALL
                SELECT tombstone.record_id
                FROM operational_fact_tombstones tombstone
                JOIN planned_routes fact
                  ON tombstone.fact_type = 'planned_routes'
                 AND fact.operator_id = tombstone.operator_id
                 AND fact.id = tombstone.record_id
                WHERE tombstone.operator_id = $1
                UNION ALL
                SELECT tombstone.record_id
                FROM operational_fact_tombstones tombstone
                JOIN weather_hazards fact
                  ON tombstone.fact_type = 'weather_hazards'
                 AND fact.operator_id = tombstone.operator_id
                 AND fact.id = tombstone.record_id
                WHERE tombstone.operator_id = $1
            )
            SELECT
                (SELECT COUNT(*)
                 FROM data_deletion_tombstones tombstone
                 JOIN provider_envelopes envelope
                   ON envelope.operator_id = tombstone.operator_id
                  AND envelope.provider = tombstone.provider
                  AND envelope.feed = tombstone.feed
                  AND envelope.raw_payload_sha256 = tombstone.raw_payload_sha256
                 WHERE tombstone.operator_id = $1
                   AND (envelope.raw_payload <> '{}'::jsonb
                        OR envelope.raw_payload_deleted_at IS NULL)) AS raw_payload_violations,
                (SELECT COUNT(*)
                 FROM lifecycle_deletion_tombstones tombstone
                 JOIN authorization_audit_events event
                   ON event.operator_id = tombstone.operator_id
                  AND event.id = tombstone.record_id
                 WHERE tombstone.operator_id = $1
                   AND tombstone.data_class = 'authorization_audit') AS authorization_audit_violations,
                (SELECT COUNT(*)
                 FROM lifecycle_deletion_tombstones tombstone
                 JOIN auth_session_revocations revocation
                   ON revocation.operator_id = tombstone.operator_id
                  AND revocation.id = tombstone.record_id
                 WHERE tombstone.operator_id = $1
                   AND tombstone.data_class = 'session_revocation') AS session_revocation_violations,
                (SELECT COUNT(*)
                 FROM lifecycle_deletion_tombstones tombstone
                 JOIN auth_identities identity ON identity.id = tombstone.record_id
                 WHERE tombstone.operator_id = $1
                   AND tombstone.data_class = 'identity_mapping'
                   AND (identity.subject <> 'deleted:' || identity.id::text
                        OR identity.display_name IS NOT NULL
                        OR identity.disabled_at IS DISTINCT FROM tombstone.deleted_at)) AS identity_mapping_violations,
                (SELECT COUNT(DISTINCT alert.id)
                 FROM alert_history_tombstones tombstone
                 JOIN alerts alert
                   ON alert.operator_id = tombstone.operator_id
                  AND (alert.id = tombstone.alert_id
                       OR alert.dedupe_key = tombstone.dedupe_key
                       OR (alert.series_key = tombstone.series_key
                           AND alert.alert_revision = tombstone.alert_revision))
                 WHERE tombstone.operator_id = $1) AS alert_history_violations,
                (SELECT COUNT(*) FROM operational_fact_violations) AS operational_fact_violations,
                (SELECT COUNT(*) FROM data_deletion_tombstones WHERE operator_id = $1) AS raw_payload_tombstones,
                (SELECT COUNT(*) FROM lifecycle_deletion_tombstones WHERE operator_id = $1) AS lifecycle_tombstones,
                (SELECT COUNT(*) FROM alert_history_tombstones WHERE operator_id = $1) AS alert_history_tombstones,
                (SELECT COUNT(*) FROM operational_fact_tombstones WHERE operator_id = $1) AS operational_fact_tombstones,
                (SELECT COUNT(*) FROM retention_schedules
                 WHERE operator_id = $1 AND status = 'paused' AND last_error_code IS NOT NULL) AS paused_schedules,
                (SELECT COUNT(*) FROM retention_schedule_attempts
                 WHERE operator_id = $1 AND status = 'failed' AND attempted_at >= $2) AS failed_attempts_24h
            "#,
        )
        .bind(operator_id.as_uuid())
        .bind(evaluated_at - Duration::hours(24))
        .fetch_one(&self.database)
        .await?;
        let violations = RetentionIntegrityViolations {
            raw_payloads: row.raw_payload_violations,
            authorization_audit: row.authorization_audit_violations,
            session_revocations: row.session_revocation_violations,
            identity_mappings: row.identity_mapping_violations,
            alert_history: row.alert_history_violations,
            operational_facts: row.operational_fact_violations,
        };
        Ok(RetentionIntegrityView {
            operator_id: operator_id.as_uuid(),
            evaluated_at,
            healthy: violations.total() == 0,
            violations,
            tombstones: RetentionTombstoneCounts {
                raw_payloads: row.raw_payload_tombstones,
                lifecycle: row.lifecycle_tombstones,
                alert_history: row.alert_history_tombstones,
                operational_facts: row.operational_fact_tombstones,
            },
            paused_schedules: row.paused_schedules,
            failed_attempts_24h: row.failed_attempts_24h,
        })
    }
}

pub(super) fn integrity_router() -> Router<RetentionStore> {
    Router::new().route("/api/admin/retention/integrity", get(integrity))
}

async fn integrity(
    State(store): State<RetentionStore>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<RetentionIntegrityView>, RetentionHttpError> {
    authorize(&context)?;
    Ok(Json(
        store
            .retention_integrity(context.operator_id, Utc::now())
            .await?,
    ))
}
