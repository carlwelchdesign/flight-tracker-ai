use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    routing::{get, patch, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    AuthContext, AuthFailure, AuthService, MembershipUpdate, MembershipView, Permission,
    SessionRevocation, service::require,
};

#[derive(Serialize)]
struct MembershipList {
    data: Vec<MembershipView>,
}

#[derive(Deserialize)]
struct RevokeSessionRequest {
    provider: String,
    session_id: String,
    identity_id: Uuid,
    reason: String,
    expires_at: DateTime<Utc>,
}

pub fn auth_router(service: AuthService) -> Router {
    Router::new()
        .route("/api/auth/context", get(context))
        .route("/api/admin/memberships", get(list_memberships))
        .route(
            "/api/admin/memberships/{membership_id}",
            patch(update_membership),
        )
        .route("/api/admin/sessions/revoke", post(revoke_session))
        .with_state(service)
}

async fn context(Extension(context): Extension<AuthContext>) -> Json<AuthContext> {
    Json(context)
}

async fn list_memberships(
    State(service): State<AuthService>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<MembershipList>, AuthFailure> {
    require(&context, Permission::ManageMemberships)?;
    let data = service
        .store()
        .list_memberships(context.operator_id)
        .await
        .map_err(|_| AuthFailure::Unavailable)?;
    Ok(Json(MembershipList { data }))
}

async fn update_membership(
    State(service): State<AuthService>,
    Extension(context): Extension<AuthContext>,
    Path(membership_id): Path<Uuid>,
    Json(update): Json<MembershipUpdate>,
) -> Result<Json<MembershipView>, AuthFailure> {
    require(&context, Permission::ManageMemberships)?;
    service
        .store()
        .update_membership(&context, membership_id, &update, Utc::now())
        .await
        .map(Json)
        .map_err(|error| match error {
            super::store::AuthStoreError::MembershipNotFound => AuthFailure::Forbidden,
            super::store::AuthStoreError::SelfLockout => AuthFailure::Forbidden,
            _ => AuthFailure::Unavailable,
        })
}

async fn revoke_session(
    State(service): State<AuthService>,
    Extension(context): Extension<AuthContext>,
    Json(request): Json<RevokeSessionRequest>,
) -> Result<Json<serde_json::Value>, AuthFailure> {
    require(&context, Permission::ManageMemberships)?;
    if request.reason.trim().is_empty()
        || request.provider.trim().is_empty()
        || request.session_id.trim().is_empty()
    {
        return Err(AuthFailure::Forbidden);
    }
    service
        .store()
        .revoke_session(
            &context,
            &SessionRevocation {
                provider: request.provider,
                session_id: request.session_id,
                identity_id: request.identity_id,
                reason: request.reason,
                expires_at: request.expires_at,
                requested_at: Utc::now(),
            },
        )
        .await
        .map_err(|error| match error {
            super::store::AuthStoreError::InvalidRevocationExpiry => AuthFailure::Forbidden,
            _ => AuthFailure::Unavailable,
        })?;
    Ok(Json(serde_json::json!({ "status": "revoked" })))
}
