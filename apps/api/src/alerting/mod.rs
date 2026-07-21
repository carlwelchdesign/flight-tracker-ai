//! Deterministic operational alert policy.
//!
//! Rule evaluation is provider-, persistence-, and web-framework-independent.
//! Alert ranking and lifecycle transitions remain pure policies; SQL and HTTP
//! adapters consume their decisions at the application boundary.

mod http;
mod lifecycle;
mod ranking;
mod route_hazard;
mod store;
mod worker;

pub use http::alert_router;
pub use lifecycle::{LifecycleError, transition_lifecycle};
pub use ranking::{
    ALERT_SCORE_VERSION, AlertCandidate, AttentionBreakdown, candidate_from_route_hazard,
};
pub use store::{
    AlertActionRequest, AlertAssignee, AlertDetail, AlertQueueFilter, AlertQueueItem, AlertStore,
    AlertStoreError, AssignmentFilter, CreateAlertResult, DismissalReason,
};
pub use worker::spawn_alert_worker;

pub use route_hazard::{
    AltitudeRelation, HazardTemporalState, HorizontalRelation, ROUTE_HAZARD_RULE_ID,
    ROUTE_HAZARD_RULE_VERSION, RouteHazardDecision, RouteHazardEvidence, RouteHazardInput,
    RouteHazardOutcome, RouteHazardRule, RouteHazardRuleConfig, RouteProgress, RouteProgressError,
    RouteTemporalState, RuleConfigError, RuleInputError, TemporalRelation,
};
