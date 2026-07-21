use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::OperatorId;

use super::{AssertionClaims, AuthContext, AuthRole};

const MAX_REVOCATION_PROVIDER_CHARS: usize = 64;
const MAX_REVOCATION_SESSION_ID_CHARS: usize = 256;
const MAX_REVOCATION_REASON_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentIdentity {
    pub operator_id: OperatorId,
    pub operator_code: String,
    pub operator_name: String,
    pub external_tenant_id: String,
    pub subject: String,
    pub display_name: String,
    pub role: AuthRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct MembershipView {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub provider: String,
    pub subject: String,
    pub display_name: Option<String>,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MembershipUpdate {
    pub role: AuthRole,
    pub status: MembershipStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipStatus {
    Active,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRevocation {
    pub provider: String,
    pub session_id: String,
    pub identity_id: Uuid,
    pub reason: String,
    pub expires_at: DateTime<Utc>,
    pub requested_at: DateTime<Utc>,
}

impl MembershipStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthStoreError {
    #[error("authorization persistence failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("identity is not an active member of the requested tenant")]
    MembershipDenied,
    #[error("stored authorization role is invalid")]
    InvalidStoredRole,
    #[error("membership was not found")]
    MembershipNotFound,
    #[error("an administrator cannot revoke or demote their own active membership")]
    SelfLockout,
    #[error("revocation expiry must be later than the current time")]
    InvalidRevocationExpiry,
    #[error("revocation provider, session ID, or reason is empty or exceeds its supported length")]
    InvalidRevocationInput,
}

#[derive(Clone)]
pub struct AuthStore {
    database: PgPool,
}

#[derive(sqlx::FromRow)]
struct ContextRow {
    identity_id: Uuid,
    operator_id: Uuid,
    operator_code: String,
    operator_name: String,
    role: String,
}

impl AuthStore {
    pub fn new(database: PgPool) -> Self {
        Self { database }
    }

    pub async fn resolve(&self, claims: &AssertionClaims) -> Result<AuthContext, AuthStoreError> {
        let row = sqlx::query_as::<_, ContextRow>(
            r#"
            SELECT identity.id AS identity_id, operator.id AS operator_id,
                   operator.code AS operator_code, operator.display_name AS operator_name,
                   membership.role
            FROM auth_identities identity
            JOIN operator_memberships membership
              ON membership.identity_id = identity.id
             AND membership.status = 'active'
            JOIN operators operator
              ON operator.id = membership.operator_id
             AND operator.identity_provider = identity.provider
             AND operator.external_tenant_id = $3
            WHERE identity.provider = $1
              AND identity.subject = $2
              AND identity.disabled_at IS NULL
              AND NOT EXISTS (
                  SELECT 1
                  FROM auth_session_revocations revoked
                  WHERE revoked.provider = identity.provider
                    AND revoked.session_id = $4
                    AND revoked.expires_at > NOW()
              )
            "#,
        )
        .bind(&claims.provider)
        .bind(&claims.sub)
        .bind(&claims.tenant)
        .bind(&claims.sid)
        .fetch_optional(&self.database)
        .await?
        .ok_or(AuthStoreError::MembershipDenied)?;
        let role =
            AuthRole::try_from(row.role.as_str()).map_err(|_| AuthStoreError::InvalidStoredRole)?;
        Ok(AuthContext {
            identity_id: row.identity_id,
            operator_id: OperatorId::from_uuid(row.operator_id),
            operator_code: row.operator_code,
            operator_name: row.operator_name,
            provider: claims.provider.clone(),
            subject: claims.sub.clone(),
            session_id: claims.sid.clone(),
            role,
        })
    }

    pub async fn bootstrap_development(
        &self,
        identity: &DevelopmentIdentity,
    ) -> Result<(), AuthStoreError> {
        let mut transaction = self.database.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO operators (
                id, code, display_name, identity_provider, external_tenant_id
            ) VALUES ($1, $2, $3, 'development', $4)
            ON CONFLICT (id) DO UPDATE SET
                identity_provider = EXCLUDED.identity_provider,
                external_tenant_id = EXCLUDED.external_tenant_id
            "#,
        )
        .bind(identity.operator_id.as_uuid())
        .bind(&identity.operator_code)
        .bind(&identity.operator_name)
        .bind(&identity.external_tenant_id)
        .execute(&mut *transaction)
        .await?;
        let identity_id = Uuid::new_v5(
            &identity.operator_id.as_uuid(),
            format!("development:{}", identity.subject).as_bytes(),
        );
        let identity_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO auth_identities (id, provider, subject, display_name)
            VALUES ($1, 'development', $2, $3)
            ON CONFLICT (provider, subject) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                disabled_at = NULL
            RETURNING id
            "#,
        )
        .bind(identity_id)
        .bind(&identity.subject)
        .bind(&identity.display_name)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO operator_memberships (
                id, operator_id, identity_id, role, status
            ) VALUES ($1, $2, $3, $4, 'active')
            ON CONFLICT (operator_id, identity_id) DO UPDATE SET
                role = EXCLUDED.role,
                status = 'active',
                revoked_at = NULL,
                updated_at = NOW()
            "#,
        )
        .bind(Uuid::new_v5(
            &identity_id,
            identity.operator_id.as_uuid().as_bytes(),
        ))
        .bind(identity.operator_id.as_uuid())
        .bind(identity_id)
        .bind(identity.role.as_str())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_memberships(
        &self,
        operator_id: OperatorId,
    ) -> Result<Vec<MembershipView>, AuthStoreError> {
        Ok(sqlx::query_as::<_, MembershipView>(
            r#"
            SELECT membership.id, identity.id AS identity_id, identity.provider,
                   identity.subject, identity.display_name, membership.role,
                   membership.status, membership.created_at, membership.updated_at,
                   membership.revoked_at
            FROM operator_memberships membership
            JOIN auth_identities identity ON identity.id = membership.identity_id
            WHERE membership.operator_id = $1
            ORDER BY identity.display_name NULLS LAST, identity.subject, membership.id
            "#,
        )
        .bind(operator_id.as_uuid())
        .fetch_all(&self.database)
        .await?)
    }

    pub async fn update_membership(
        &self,
        actor: &AuthContext,
        membership_id: Uuid,
        update: &MembershipUpdate,
        now: DateTime<Utc>,
    ) -> Result<MembershipView, AuthStoreError> {
        let mut transaction = self.database.begin().await?;
        let target_identity = sqlx::query_scalar::<_, Uuid>(
            "SELECT identity_id FROM operator_memberships WHERE operator_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(actor.operator_id.as_uuid())
        .bind(membership_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(AuthStoreError::MembershipNotFound)?;
        if target_identity == actor.identity_id
            && (update.status == MembershipStatus::Revoked
                || update.role != AuthRole::Administrator)
        {
            return Err(AuthStoreError::SelfLockout);
        }
        sqlx::query(
            r#"
            UPDATE operator_memberships
            SET role = $3, status = $4, updated_at = $5,
                revoked_at = CASE WHEN $4 = 'revoked' THEN $5 ELSE NULL END
            WHERE operator_id = $1 AND id = $2
            "#,
        )
        .bind(actor.operator_id.as_uuid())
        .bind(membership_id)
        .bind(update.role.as_str())
        .bind(update.status.as_str())
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        insert_audit(
            &mut transaction,
            actor,
            "membership.updated",
            "operator_membership",
            &membership_id.to_string(),
            json!({"role": update.role, "status": update.status}),
            now,
        )
        .await?;
        transaction.commit().await?;
        self.membership(actor.operator_id, membership_id).await
    }

    async fn membership(
        &self,
        operator_id: OperatorId,
        membership_id: Uuid,
    ) -> Result<MembershipView, AuthStoreError> {
        sqlx::query_as::<_, MembershipView>(
            r#"
            SELECT membership.id, identity.id AS identity_id, identity.provider,
                   identity.subject, identity.display_name, membership.role,
                   membership.status, membership.created_at, membership.updated_at,
                   membership.revoked_at
            FROM operator_memberships membership
            JOIN auth_identities identity ON identity.id = membership.identity_id
            WHERE membership.operator_id = $1 AND membership.id = $2
            "#,
        )
        .bind(operator_id.as_uuid())
        .bind(membership_id)
        .fetch_optional(&self.database)
        .await?
        .ok_or(AuthStoreError::MembershipNotFound)
    }

    pub async fn revoke_session(
        &self,
        actor: &AuthContext,
        request: &SessionRevocation,
    ) -> Result<(), AuthStoreError> {
        let provider = bounded_revocation_field(&request.provider, MAX_REVOCATION_PROVIDER_CHARS)?;
        let session_id =
            bounded_revocation_field(&request.session_id, MAX_REVOCATION_SESSION_ID_CHARS)?;
        let reason = bounded_revocation_field(&request.reason, MAX_REVOCATION_REASON_CHARS)?;
        if request.expires_at <= request.requested_at {
            return Err(AuthStoreError::InvalidRevocationExpiry);
        }
        let mut transaction = self.database.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO auth_session_revocations (
                id, provider, session_id, identity_id, operator_id,
                revoked_by_identity_id, reason, revoked_at, expires_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (provider, session_id) DO UPDATE SET
                reason = EXCLUDED.reason,
                revoked_by_identity_id = EXCLUDED.revoked_by_identity_id,
                revoked_at = EXCLUDED.revoked_at,
                expires_at = GREATEST(auth_session_revocations.expires_at, EXCLUDED.expires_at)
            "#,
        )
        .bind(Uuid::new_v5(
            &actor.operator_id.as_uuid(),
            format!("{provider}:{session_id}").as_bytes(),
        ))
        .bind(provider)
        .bind(session_id)
        .bind(request.identity_id)
        .bind(actor.operator_id.as_uuid())
        .bind(actor.identity_id)
        .bind(reason)
        .bind(request.requested_at)
        .bind(request.expires_at)
        .execute(&mut *transaction)
        .await?;
        insert_audit(
            &mut transaction,
            actor,
            "session.revoked",
            "auth_session",
            session_id,
            json!({"provider": provider, "identity_id": request.identity_id, "reason": reason}),
            request.requested_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}

fn bounded_revocation_field(value: &str, max_chars: usize) -> Result<&str, AuthStoreError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars {
        return Err(AuthStoreError::InvalidRevocationInput);
    }
    Ok(value)
}

async fn insert_audit(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &AuthContext,
    action: &str,
    target_type: &str,
    target_id: &str,
    metadata: serde_json::Value,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO authorization_audit_events (
            id, operator_id, actor_identity_id, action, target_type,
            target_id, occurred_at, metadata
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(actor.operator_id.as_uuid())
    .bind(actor.identity_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(now)
    .bind(metadata)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revocation_fields_are_trimmed_and_bounded_by_characters() {
        assert_eq!(bounded_revocation_field(" reason ", 6).unwrap(), "reason");
        assert!(bounded_revocation_field("   ", 500).is_err());
        assert!(bounded_revocation_field(&"x".repeat(501), 500).is_err());
        assert_eq!(bounded_revocation_field("éé", 2).unwrap(), "éé");
    }
}
