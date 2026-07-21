//! Provider-neutral request authentication and app-owned authorization.
//!
//! Hosted identity adapters terminate in the Next.js BFF. Rust accepts only a
//! short-lived signed assertion, then resolves tenant membership and role from
//! PostgreSQL before an operational handler runs.

mod http;
mod model;
mod service;
mod store;
mod token;

pub use http::auth_router;
pub use model::{AuthContext, AuthRole, Permission};
pub use service::{AuthFailure, AuthService, authenticate_request, require};
pub use store::{
    AuthStore, DevelopmentIdentity, MembershipStatus, MembershipUpdate, MembershipView,
    SessionRevocation,
};
pub use token::{AssertionClaims, AssertionConfig, AssertionError, InternalAssertionVerifier};
