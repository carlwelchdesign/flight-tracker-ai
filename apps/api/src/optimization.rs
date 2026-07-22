//! Offline, fixture-only route-candidate recommendation experiment.
//!
//! This module has no HTTP, database, provider, messaging, or control adapter.
//! It ranks project-authored candidates for human review and can abstain.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    alerting::{RouteHazardInput, RouteHazardOutcome, RouteHazardRule, RouteHazardRuleConfig},
    domain::{
        Altitude, AltitudeBand, AltitudeReference, AltitudeUnit, EventTimes, FlightId,
        GeoLineString, GeoPoint, GeoPolygon, HazardSeverity, OperatorId, PlannedRoute,
        PlannedRouteId, ProviderEnvelopeId, SchemaVersion, SourceAttribution, WeatherHazard,
        WeatherHazardId, WeatherHazardStatus,
    },
};

pub const EXPERIMENT_ID: &str = "offline_route_candidate_ranking";
pub const RULE_VERSION: u32 = 1;
const METERS_PER_NAUTICAL_MILE: f64 = 1_852.0;
const EARTH_RADIUS_METERS: f64 = 6_371_008.8;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentDataset {
    pub schema_version: u32,
    pub dataset_version: String,
    pub config: ExperimentConfig,
    pub route_templates: BTreeMap<String, RouteTemplate>,
    pub hazard_templates: BTreeMap<String, HazardTemplate>,
    pub cases: Vec<ExperimentCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExperimentConfig {
    pub rule_version: u32,
    pub proximity_margin_nm: f64,
    pub geometry_resolution_nm: f64,
    pub max_added_distance_percent: f64,
    pub endpoint_tolerance_nm: f64,
    pub max_candidates: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteTemplate {
    pub coordinates: Vec<GeoPoint>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardTemplate {
    pub exterior: Vec<GeoPoint>,
    pub lower_feet_msl: i32,
    pub upper_feet_msl: i32,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSplit {
    Development,
    HeldOut,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedDisposition {
    Recommend,
    Abstain,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentCase {
    pub id: String,
    pub split: DatasetSplit,
    pub category: String,
    pub evaluated_at: DateTime<Utc>,
    pub origin_code: String,
    pub destination_code: String,
    pub origin: GeoPoint,
    pub destination: GeoPoint,
    pub aircraft_altitude_feet_msl: Option<i32>,
    pub evidence_complete: bool,
    pub hazard_template: String,
    pub candidate_templates: Vec<String>,
    pub expected_disposition: ExpectedDisposition,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExperimentResult {
    pub experiment_id: &'static str,
    pub dataset_version: String,
    pub case_id: String,
    pub split: DatasetSplit,
    pub evaluated_at: DateTime<Utc>,
    pub rule_version: u32,
    pub config: ExperimentConfig,
    pub inputs: InputEvidence,
    pub baseline: Option<BaselineResult>,
    pub candidates: Vec<CandidateAssessment>,
    pub outcome: RecommendationOutcome,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InputEvidence {
    pub origin_code: String,
    pub destination_code: String,
    pub aircraft_altitude_feet_msl: Option<i32>,
    pub hazard_template: String,
    pub candidate_templates: Vec<String>,
    pub source: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BaselineResult {
    pub candidate_id: String,
    pub distance_nm: f64,
    pub hazard_clear: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CandidateAssessment {
    pub candidate_id: String,
    pub distance_nm: Option<f64>,
    pub added_distance_nm: Option<f64>,
    pub added_distance_percent: Option<f64>,
    pub segment_count: usize,
    pub closest_approach_nm: Option<f64>,
    pub constraints: Vec<ConstraintResult>,
    pub eligible: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConstraintResult {
    pub id: &'static str,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecommendationOutcome {
    Recommendation {
        candidate_id: String,
        label: &'static str,
        expected_effect: ExpectedEffect,
        required_action: &'static str,
    },
    Abstention {
        reasons: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExpectedEffect {
    pub hazard_clear: bool,
    pub closest_approach_nm: f64,
    pub added_distance_nm: f64,
    pub added_distance_percent: f64,
    pub disclaimer: &'static str,
}

#[derive(Debug, Error)]
pub enum ExperimentError {
    #[error("invalid experiment dataset: {0}")]
    InvalidDataset(String),
    #[error("case {case_id} references missing {kind} template {template_id}")]
    MissingTemplate {
        case_id: String,
        kind: &'static str,
        template_id: String,
    },
}

pub fn load_dataset(json: &str) -> Result<ExperimentDataset, ExperimentError> {
    let dataset: ExperimentDataset = serde_json::from_str(json)
        .map_err(|error| ExperimentError::InvalidDataset(error.to_string()))?;
    validate_dataset(&dataset)?;
    Ok(dataset)
}

pub fn evaluate_dataset(
    dataset: &ExperimentDataset,
) -> Result<Vec<ExperimentResult>, ExperimentError> {
    dataset
        .cases
        .iter()
        .map(|case| evaluate_case(dataset, case))
        .collect()
}

pub fn evaluate_case(
    dataset: &ExperimentDataset,
    case: &ExperimentCase,
) -> Result<ExperimentResult, ExperimentError> {
    let inputs = InputEvidence {
        origin_code: case.origin_code.clone(),
        destination_code: case.destination_code.clone(),
        aircraft_altitude_feet_msl: case.aircraft_altitude_feet_msl,
        hazard_template: case.hazard_template.clone(),
        candidate_templates: case.candidate_templates.clone(),
        source: "project-authored synthetic replay fixture",
    };
    let mut abstention_reasons = Vec::new();
    if !case.evidence_complete {
        abstention_reasons.push("required evidence is marked incomplete".to_owned());
    }
    if case.aircraft_altitude_feet_msl.is_none() {
        abstention_reasons.push("aircraft altitude context is missing".to_owned());
    }
    if case.candidate_templates.is_empty() {
        abstention_reasons.push("no pre-authored candidates are available".to_owned());
    }

    let hazard = dataset
        .hazard_templates
        .get(&case.hazard_template)
        .ok_or_else(|| ExperimentError::MissingTemplate {
            case_id: case.id.clone(),
            kind: "hazard",
            template_id: case.hazard_template.clone(),
        })?;
    let direct_distance_nm = distance_nm(case.origin, case.destination);
    let mut assessments = Vec::with_capacity(case.candidate_templates.len());
    for candidate_id in &case.candidate_templates {
        let route = dataset.route_templates.get(candidate_id).ok_or_else(|| {
            ExperimentError::MissingTemplate {
                case_id: case.id.clone(),
                kind: "route",
                template_id: candidate_id.clone(),
            }
        })?;
        assessments.push(assess_candidate(
            dataset,
            case,
            candidate_id,
            route,
            hazard,
            direct_distance_nm,
        ));
    }

    let baseline = assessments
        .iter()
        .filter_map(|candidate| Some((candidate, candidate.distance_nm?)))
        .min_by(|(left, left_distance), (right, right_distance)| {
            left_distance
                .total_cmp(right_distance)
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        })
        .map(|(candidate, distance_nm)| BaselineResult {
            candidate_id: candidate.candidate_id.clone(),
            distance_nm,
            hazard_clear: candidate
                .constraints
                .iter()
                .find(|constraint| constraint.id == "hazard_clearance")
                .is_some_and(|constraint| constraint.passed),
        });

    let winner = assessments
        .iter()
        .filter(|candidate| candidate.eligible)
        .min_by(|left, right| {
            left.added_distance_nm
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.added_distance_nm.unwrap_or(f64::INFINITY))
                .then_with(|| left.segment_count.cmp(&right.segment_count))
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });

    let outcome = if !abstention_reasons.is_empty() {
        RecommendationOutcome::Abstention {
            reasons: abstention_reasons,
        }
    } else if let Some(winner) = winner {
        RecommendationOutcome::Recommendation {
            candidate_id: winner.candidate_id.clone(),
            label: "offline recommendation for review",
            expected_effect: ExpectedEffect {
                hazard_clear: true,
                closest_approach_nm: winner.closest_approach_nm.unwrap_or(0.0),
                added_distance_nm: winner.added_distance_nm.unwrap_or(0.0),
                added_distance_percent: winner.added_distance_percent.unwrap_or(0.0),
                disclaimer: "Geometric fixture proxy only; not fuel, time, cost, flight planning, or operational safety advice.",
            },
            required_action: "A human reviewer must accept, reject, or annotate; no external action is available.",
        }
    } else {
        RecommendationOutcome::Abstention {
            reasons: vec!["no candidate satisfies every hard constraint".to_owned()],
        }
    };

    Ok(ExperimentResult {
        experiment_id: EXPERIMENT_ID,
        dataset_version: dataset.dataset_version.clone(),
        case_id: case.id.clone(),
        split: case.split,
        evaluated_at: case.evaluated_at,
        rule_version: RULE_VERSION,
        config: dataset.config.clone(),
        inputs,
        baseline,
        candidates: assessments,
        outcome,
    })
}

fn validate_dataset(dataset: &ExperimentDataset) -> Result<(), ExperimentError> {
    if dataset.schema_version != 1 || dataset.config.rule_version != RULE_VERSION {
        return Err(ExperimentError::InvalidDataset(
            "unsupported schema or rule version".to_owned(),
        ));
    }
    if dataset.dataset_version.trim().is_empty() {
        return Err(ExperimentError::InvalidDataset(
            "dataset version is required".to_owned(),
        ));
    }
    let development = dataset
        .cases
        .iter()
        .filter(|case| case.split == DatasetSplit::Development)
        .count();
    let held_out = dataset
        .cases
        .iter()
        .filter(|case| case.split == DatasetSplit::HeldOut)
        .count();
    if development != 18 || held_out != 12 {
        return Err(ExperimentError::InvalidDataset(format!(
            "expected 18 development and 12 held-out cases, found {development} and {held_out}"
        )));
    }
    if dataset.config.max_candidates == 0 || dataset.config.max_candidates > 12 {
        return Err(ExperimentError::InvalidDataset(
            "max_candidates must be between 1 and 12".to_owned(),
        ));
    }
    RouteHazardRule::new(RouteHazardRuleConfig {
        proximity_margin_nm: dataset.config.proximity_margin_nm,
        geometry_resolution_nm: dataset.config.geometry_resolution_nm,
    })
    .map_err(|error| ExperimentError::InvalidDataset(error.to_string()))?;
    if !dataset.config.max_added_distance_percent.is_finite()
        || !(0.0..=25.0).contains(&dataset.config.max_added_distance_percent)
        || !dataset.config.endpoint_tolerance_nm.is_finite()
        || !(0.0..=1.0).contains(&dataset.config.endpoint_tolerance_nm)
    {
        return Err(ExperimentError::InvalidDataset(
            "distance and endpoint bounds exceed the approved experiment".to_owned(),
        ));
    }
    if dataset
        .cases
        .iter()
        .any(|case| case.candidate_templates.len() > dataset.config.max_candidates)
    {
        return Err(ExperimentError::InvalidDataset(
            "case exceeds configured candidate bound".to_owned(),
        ));
    }
    if dataset.cases.iter().any(|case| {
        !distance_nm(case.origin, case.destination).is_finite()
            || distance_nm(case.origin, case.destination) <= f64::EPSILON
    }) {
        return Err(ExperimentError::InvalidDataset(
            "every case requires distinct finite origin and destination points".to_owned(),
        ));
    }
    Ok(())
}

fn assess_candidate(
    dataset: &ExperimentDataset,
    case: &ExperimentCase,
    candidate_id: &str,
    template: &RouteTemplate,
    hazard_template: &HazardTemplate,
    direct_distance_nm: f64,
) -> CandidateAssessment {
    let mut constraints = Vec::new();
    let point_count_valid = (2..=32).contains(&template.coordinates.len());
    constraints.push(constraint(
        "bounded_geometry",
        point_count_valid,
        format!("{} route points; allowed 2–32", template.coordinates.len()),
    ));
    let endpoints_valid = template.coordinates.first().is_some_and(|point| {
        distance_nm(*point, case.origin) <= dataset.config.endpoint_tolerance_nm
    }) && template.coordinates.last().is_some_and(|point| {
        distance_nm(*point, case.destination) <= dataset.config.endpoint_tolerance_nm
    });
    constraints.push(constraint(
        "matching_endpoints",
        endpoints_valid,
        format!(
            "{} to {} fixture endpoints",
            case.origin_code, case.destination_code
        ),
    ));
    let distance = point_count_valid.then(|| path_distance_nm(&template.coordinates));
    let added_nm = distance.map(|value| (value - direct_distance_nm).max(0.0));
    let added_percent = added_nm.map(|value| value / direct_distance_nm * 100.0);
    let distance_valid = added_percent.is_some_and(|value| {
        value.is_finite() && value <= dataset.config.max_added_distance_percent
    });
    constraints.push(constraint(
        "added_distance_bound",
        distance_valid,
        format!(
            "{:.3}% added; maximum {:.3}%",
            added_percent.unwrap_or(f64::INFINITY),
            dataset.config.max_added_distance_percent
        ),
    ));
    let evidence_valid = case.evidence_complete && case.aircraft_altitude_feet_msl.is_some();
    constraints.push(constraint(
        "complete_evidence",
        evidence_valid,
        "versioned route, hazard, time, units, and altitude context".to_owned(),
    ));

    let decision = if point_count_valid && endpoints_valid && evidence_valid {
        evaluate_geometry(dataset, case, candidate_id, template, hazard_template).ok()
    } else {
        None
    };
    let hazard_clear = decision
        .as_ref()
        .is_some_and(|decision| decision.outcome == RouteHazardOutcome::NoMatch);
    let closest_approach_nm = decision
        .as_ref()
        .map(|decision| decision.evidence.closest_approach_nm);
    constraints.push(constraint(
        "hazard_clearance",
        hazard_clear,
        decision.map_or_else(
            || "not evaluated because prerequisite evidence is invalid".to_owned(),
            |value| {
                format!(
                    "{:?}; closest approach {:.3} NM; fixed margin {:.3} NM",
                    value.evidence.horizontal_relation,
                    value.evidence.closest_approach_nm,
                    value.evidence.proximity_margin_nm
                )
            },
        ),
    ));
    let eligible = constraints.iter().all(|constraint| constraint.passed);
    CandidateAssessment {
        candidate_id: candidate_id.to_owned(),
        distance_nm: distance.map(quantize),
        added_distance_nm: added_nm.map(quantize),
        added_distance_percent: added_percent.map(quantize),
        segment_count: template.coordinates.len().saturating_sub(1),
        closest_approach_nm,
        constraints,
        eligible,
    }
}

fn evaluate_geometry(
    dataset: &ExperimentDataset,
    case: &ExperimentCase,
    candidate_id: &str,
    template: &RouteTemplate,
    hazard_template: &HazardTemplate,
) -> Result<crate::alerting::RouteHazardDecision, crate::alerting::RuleInputError> {
    let operator_id = OperatorId::from_uuid(stable_uuid("operator"));
    let flight_id = FlightId::from_uuid(stable_uuid(&case.id));
    let source = SourceAttribution {
        envelope_id: ProviderEnvelopeId::from_uuid(stable_uuid("fixture-envelope")),
        provider: "portfolio-fixture".to_owned(),
        feed: dataset.dataset_version.clone(),
        provider_record_id: Some(case.id.clone()),
    };
    let times = EventTimes {
        event_time: case.evaluated_at,
        received_at: case.evaluated_at,
        processed_at: case.evaluated_at,
    };
    let route = PlannedRoute {
        id: PlannedRouteId::from_uuid(stable_uuid(candidate_id)),
        operator_id,
        flight_id,
        schema_version: SchemaVersion::V1,
        source: source.clone(),
        times: times.clone(),
        route_version: RULE_VERSION,
        effective_from: case.evaluated_at,
        effective_to: Some(case.evaluated_at + chrono::Duration::hours(1)),
        path: GeoLineString {
            coordinates: template.coordinates.clone(),
        },
    };
    let hazard = WeatherHazard {
        id: WeatherHazardId::from_uuid(stable_uuid(&case.hazard_template)),
        operator_id,
        schema_version: SchemaVersion::V1,
        source,
        times,
        external_series_id: case.hazard_template.clone(),
        revision: 1,
        supersedes_id: None,
        status: WeatherHazardStatus::Active,
        issued_at: case.evaluated_at,
        provider_received_at: None,
        hazard_type: "synthetic_convective_fixture".to_owned(),
        severity: HazardSeverity::Significant,
        valid_from: case.evaluated_at,
        valid_to: case.evaluated_at + chrono::Duration::hours(1),
        altitude_band: Some(AltitudeBand {
            lower: Some(altitude(hazard_template.lower_feet_msl)),
            upper: Some(altitude(hazard_template.upper_feet_msl)),
        }),
        footprint: GeoPolygon {
            exterior: hazard_template.exterior.clone(),
        },
    };
    let route_band = case.aircraft_altitude_feet_msl.map(|value| AltitudeBand {
        lower: Some(altitude(value)),
        upper: Some(altitude(value)),
    });
    RouteHazardRule::new(RouteHazardRuleConfig {
        proximity_margin_nm: dataset.config.proximity_margin_nm,
        geometry_resolution_nm: dataset.config.geometry_resolution_nm,
    })
    .expect("validated experiment configuration")
    .evaluate(RouteHazardInput {
        route: &route,
        hazard: &hazard,
        evaluated_at: case.evaluated_at,
        route_altitude_band: route_band.as_ref(),
        progress: None,
    })
}

fn constraint(id: &'static str, passed: bool, detail: String) -> ConstraintResult {
    ConstraintResult { id, passed, detail }
}

fn altitude(value: i32) -> Altitude {
    Altitude {
        value,
        unit: AltitudeUnit::Feet,
        reference: AltitudeReference::MeanSeaLevel,
    }
}

fn stable_uuid(value: &str) -> uuid::Uuid {
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, value.as_bytes())
}

fn path_distance_nm(points: &[GeoPoint]) -> f64 {
    points
        .windows(2)
        .map(|segment| distance_nm(segment[0], segment[1]))
        .sum()
}

fn distance_nm(left: GeoPoint, right: GeoPoint) -> f64 {
    let [left_lon, left_lat] = left.as_geojson_position();
    let [right_lon, right_lat] = right.as_geojson_position();
    let left_lat = left_lat.to_radians();
    let right_lat = right_lat.to_radians();
    let latitude_delta = right_lat - left_lat;
    let longitude_delta = (right_lon - left_lon).to_radians();
    let haversine = (latitude_delta / 2.0).sin().powi(2)
        + left_lat.cos() * right_lat.cos() * (longitude_delta / 2.0).sin().powi(2);
    EARTH_RADIUS_METERS * 2.0 * haversine.sqrt().asin() / METERS_PER_NAUTICAL_MILE
}

fn quantize(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATASET: &str = include_str!("../../../fixtures/optimization/ft502-cases-v1.json");

    #[test]
    fn dataset_has_the_sealed_development_and_held_out_shape() {
        let dataset = load_dataset(DATASET).unwrap();
        assert_eq!(dataset.cases.len(), 30);
        assert_eq!(
            dataset
                .cases
                .iter()
                .filter(|case| case.split == DatasetSplit::Development)
                .count(),
            18
        );
        assert_eq!(
            dataset
                .cases
                .iter()
                .filter(|case| case.split == DatasetSplit::HeldOut)
                .count(),
            12
        );
    }

    #[test]
    fn every_case_matches_its_sealed_expected_disposition() {
        let dataset = load_dataset(DATASET).unwrap();
        for (case, result) in dataset
            .cases
            .iter()
            .zip(evaluate_dataset(&dataset).unwrap())
        {
            let actual = match result.outcome {
                RecommendationOutcome::Recommendation { .. } => ExpectedDisposition::Recommend,
                RecommendationOutcome::Abstention { .. } => ExpectedDisposition::Abstain,
            };
            assert_eq!(actual, case.expected_disposition, "{}", case.id);
        }
    }

    #[test]
    fn held_out_recommendations_are_constraint_clean_and_improve_on_baseline() {
        let dataset = load_dataset(DATASET).unwrap();
        let results = evaluate_dataset(&dataset).unwrap();
        let held_out: Vec<_> = results
            .iter()
            .filter(|result| result.split == DatasetSplit::HeldOut)
            .collect();
        let recommendations: Vec<_> = held_out
            .iter()
            .filter(|result| matches!(result.outcome, RecommendationOutcome::Recommendation { .. }))
            .collect();
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().all(|result| {
            result
                .candidates
                .iter()
                .filter(|candidate| candidate.eligible)
                .all(|candidate| {
                    candidate
                        .constraints
                        .iter()
                        .all(|constraint| constraint.passed)
                })
        }));
        let baseline_clear = recommendations
            .iter()
            .filter(|result| {
                result
                    .baseline
                    .as_ref()
                    .is_some_and(|baseline| baseline.hazard_clear)
            })
            .count() as f64
            / recommendations.len() as f64;
        let selected_clear = 1.0;
        assert!(selected_clear - baseline_clear >= 0.30);

        let mut added_percentages: Vec<_> = recommendations
            .iter()
            .filter_map(|result| match &result.outcome {
                RecommendationOutcome::Recommendation {
                    expected_effect, ..
                } => Some(expected_effect.added_distance_percent),
                RecommendationOutcome::Abstention { .. } => None,
            })
            .collect();
        added_percentages.sort_by(f64::total_cmp);
        let median = added_percentages[added_percentages.len() / 2];
        assert!(
            median <= 20.0,
            "held-out median added distance was {median}%"
        );
    }

    #[test]
    fn missing_and_impossible_held_out_cases_abstain() {
        let dataset = load_dataset(DATASET).unwrap();
        for (case, result) in dataset
            .cases
            .iter()
            .zip(evaluate_dataset(&dataset).unwrap())
        {
            if case.split == DatasetSplit::HeldOut
                && matches!(case.category.as_str(), "missing_evidence" | "impossible")
            {
                assert!(
                    matches!(result.outcome, RecommendationOutcome::Abstention { .. }),
                    "{}",
                    case.id
                );
            }
        }
    }

    #[test]
    fn repeated_runs_are_byte_identical() {
        let dataset = load_dataset(DATASET).unwrap();
        let first = serde_json::to_vec(&evaluate_dataset(&dataset).unwrap()).unwrap();
        let second = serde_json::to_vec(&evaluate_dataset(&dataset).unwrap()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn result_contract_has_no_delivery_or_provider_payload_fields() {
        let dataset = load_dataset(DATASET).unwrap();
        let value = serde_json::to_value(evaluate_dataset(&dataset).unwrap()).unwrap();
        let serialized = value.to_string();
        for forbidden in [
            "send",
            "dispatch",
            "clearance",
            "raw_payload",
            "provider_record_id",
        ] {
            assert!(!serialized.contains(&format!("\"{forbidden}\"")));
        }
    }
}
