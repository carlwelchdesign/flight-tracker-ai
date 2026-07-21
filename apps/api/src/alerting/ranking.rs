use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::domain::{
    AlertSeverity, FlightId, HazardSeverity, OperatorId, PlannedRoute, ProviderEnvelopeId,
    WeatherHazard, WeatherHazardId,
};

use super::{AltitudeRelation, HorizontalRelation, RouteHazardDecision, RouteHazardOutcome};

pub const ALERT_SCORE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttentionBreakdown {
    pub hazard_severity_points: u8,
    pub horizontal_proximity_points: u8,
    pub altitude_overlap_points: u8,
    pub time_urgency_points: u8,
    pub total: u8,
    pub score_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertCandidate {
    pub operator_id: OperatorId,
    pub flight_id: FlightId,
    pub hazard_id: WeatherHazardId,
    pub alert_type: String,
    pub severity: AlertSeverity,
    pub series_key: String,
    pub dedupe_key: String,
    pub attention: AttentionBreakdown,
    pub decision: RouteHazardDecision,
    pub evidence_envelope_ids: Vec<ProviderEnvelopeId>,
}

pub fn candidate_from_route_hazard(
    route: &PlannedRoute,
    hazard: &WeatherHazard,
    decision: RouteHazardDecision,
) -> Option<AlertCandidate> {
    if decision.outcome != RouteHazardOutcome::Match {
        return None;
    }

    let attention = attention_breakdown(hazard, &decision);
    let series_key = format!(
        "route_hazard:{}:{}:{}",
        route.operator_id.as_uuid(),
        route.flight_id.as_uuid(),
        hazard.external_series_id
    );
    let dedupe_key = format!(
        "{}:route-v{}:hazard-r{}:rule-v{}:urgency-p{}",
        series_key,
        route.route_version,
        hazard.revision,
        decision.evidence.rule_version,
        attention.time_urgency_points
    );
    let mut evidence_envelope_ids = vec![route.source.envelope_id, hazard.source.envelope_id];
    evidence_envelope_ids.sort_by_key(|id| id.as_uuid());
    evidence_envelope_ids.dedup();

    Some(AlertCandidate {
        operator_id: route.operator_id,
        flight_id: route.flight_id,
        hazard_id: hazard.id,
        alert_type: "route_hazard_proximity".into(),
        severity: score_severity(attention.total),
        series_key,
        dedupe_key,
        attention,
        decision,
        evidence_envelope_ids,
    })
}

fn attention_breakdown(
    hazard: &WeatherHazard,
    decision: &RouteHazardDecision,
) -> AttentionBreakdown {
    let hazard_severity_points = match hazard.severity {
        HazardSeverity::Unknown => 10,
        HazardSeverity::Advisory => 25,
        HazardSeverity::Significant => 45,
        HazardSeverity::Severe => 60,
    };
    let horizontal_proximity_points = match decision.evidence.horizontal_relation {
        HorizontalRelation::Intersects => 25,
        HorizontalRelation::WithinMargin => {
            let margin = decision.evidence.proximity_margin_nm.max(f64::EPSILON);
            let ratio = 1.0 - (decision.evidence.closest_approach_nm / margin).clamp(0.0, 1.0);
            (10.0 + ratio * 10.0).round() as u8
        }
        HorizontalRelation::Clear | HorizontalRelation::BehindRouteProgress => 0,
    };
    let altitude_overlap_points = match decision.evidence.altitude_relation {
        AltitudeRelation::Overlap => 10,
        AltitudeRelation::Disjoint | AltitudeRelation::Indeterminate => 0,
    };
    let remaining = hazard.valid_to - decision.evidence.evaluated_at;
    let time_urgency_points = if remaining <= Duration::minutes(30) {
        5
    } else {
        0
    };
    let total = (hazard_severity_points
        + horizontal_proximity_points
        + altitude_overlap_points
        + time_urgency_points)
        .min(100);
    AttentionBreakdown {
        hazard_severity_points,
        horizontal_proximity_points,
        altitude_overlap_points,
        time_urgency_points,
        total,
        score_version: ALERT_SCORE_VERSION,
    }
}

fn score_severity(score: u8) -> AlertSeverity {
    match score {
        85..=100 => AlertSeverity::Critical,
        60..=84 => AlertSeverity::Warning,
        30..=59 => AlertSeverity::Advisory,
        _ => AlertSeverity::Information,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_thresholds_are_stable() {
        assert_eq!(score_severity(29), AlertSeverity::Information);
        assert_eq!(score_severity(30), AlertSeverity::Advisory);
        assert_eq!(score_severity(60), AlertSeverity::Warning);
        assert_eq!(score_severity(85), AlertSeverity::Critical);
    }
}
