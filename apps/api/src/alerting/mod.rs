//! Deterministic operational alert policy.
//!
//! Rule evaluation is provider-, persistence-, and web-framework-independent.
//! FT-204 will consume these decisions when it adds alert lifecycle behavior.

mod route_hazard;

pub use route_hazard::{
    AltitudeRelation, HazardTemporalState, HorizontalRelation, ROUTE_HAZARD_RULE_ID,
    ROUTE_HAZARD_RULE_VERSION, RouteHazardDecision, RouteHazardEvidence, RouteHazardInput,
    RouteHazardOutcome, RouteHazardRule, RouteHazardRuleConfig, RouteProgress, RouteProgressError,
    RouteTemporalState, RuleConfigError, RuleInputError, TemporalRelation,
};
