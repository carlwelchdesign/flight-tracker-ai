use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

use super::{AuthContext, AuthStore, InternalAssertionVerifier, Permission, store::AuthStoreError};

#[derive(Clone)]
pub struct AuthService {
    verifier: InternalAssertionVerifier,
    store: AuthStore,
}

#[derive(Debug, Error)]
pub enum AuthFailure {
    #[error("a valid session is required")]
    Unauthenticated,
    #[error("the current session is not authorized for this tenant or action")]
    Forbidden,
    #[error("authorization is temporarily unavailable")]
    Unavailable,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for AuthFailure {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "A valid session is required",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "authorization_denied",
                "The current session is not authorized for this tenant or action",
            ),
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "authorization_unavailable",
                "Authorization is temporarily unavailable",
            ),
        };
        (
            status,
            Json(ErrorResponse {
                error: ErrorBody { code, message },
            }),
        )
            .into_response()
    }
}

impl AuthService {
    pub fn new(verifier: InternalAssertionVerifier, store: AuthStore) -> Self {
        Self { verifier, store }
    }

    pub fn store(&self) -> &AuthStore {
        &self.store
    }

    pub async fn authenticate(&self, authorization: &str) -> Result<AuthContext, AuthFailure> {
        let claims = self
            .verifier
            .verify_header(authorization)
            .map_err(|_| AuthFailure::Unauthenticated)?;
        self.store
            .resolve(&claims)
            .await
            .map_err(|error| match error {
                AuthStoreError::MembershipDenied | AuthStoreError::InvalidStoredRole => {
                    AuthFailure::Forbidden
                }
                _ => AuthFailure::Unavailable,
            })
    }
}

pub async fn authenticate_request(
    State(service): State<AuthService>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(value) = request.headers().get(header::AUTHORIZATION) else {
        return AuthFailure::Unauthenticated.into_response();
    };
    let Ok(value) = value.to_str() else {
        return AuthFailure::Unauthenticated.into_response();
    };
    match service.authenticate(value).await {
        Ok(context) => {
            request.extensions_mut().insert(context);
            next.run(request).await
        }
        Err(error) => error.into_response(),
    }
}

pub fn require(context: &AuthContext, permission: Permission) -> Result<(), AuthFailure> {
    if context.permits(permission) {
        Ok(())
    } else {
        Err(AuthFailure::Forbidden)
    }
}
