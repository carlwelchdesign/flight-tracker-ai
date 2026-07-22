use std::sync::LazyLock;

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::{
    alerting::{
        AttentionBreakdown, RouteHazardInput, RouteHazardRule, candidate_from_route_hazard,
    },
    domain::{
        AlertSeverity, Altitude, AltitudeBand, CanonicalEvent, FlightId, PlannedRoute,
        WeatherHazard,
    },
    replay::{ReplayScenario, ScenarioPayload},
};

pub(crate) const PORTFOLIO_SCENARIO: &str =
    include_str!("../../../fixtures/replay/m1-operations-v1.json");
const ATTENTION_CALLSIGN: &str = "FT303";

static PUBLIC_PICTURE: LazyLock<Result<PublicAttentionPicture, String>> =
    LazyLock::new(build_public_attention_picture);

#[derive(Debug, Clone, Serialize)]
pub struct PublicAttentionPicture {
    schema_version: u16,
    scenario_id: String,
    scenario_time: DateTime<Utc>,
    source: &'static str,
    aircraft: Vec<PublicAircraftAttention>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicAircraftAttention {
    callsign: String,
    state: PublicAttentionState,
    priority: Option<AlertSeverity>,
    summary: String,
    observed_facts: Vec<PublicAttentionFact>,
    score: Option<AttentionBreakdown>,
    rule_result: Option<PublicRuleResult>,
    geometric_estimate: Option<PublicGeometricEstimate>,
    source_times: PublicSourceTimes,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum PublicAttentionState {
    RequiresAttention,
    NotEvaluated,
}

#[derive(Debug, Clone, Serialize)]
struct PublicAttentionFact {
    label: &'static str,
    value: String,
}

#[derive(Debug, Clone, Serialize)]
struct PublicRuleResult {
    rule_id: String,
    rule_version: u32,
    outcome: &'static str,
    route_version: u32,
    hazard_revision: u32,
    horizontal_relation: &'static str,
    altitude_relation: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct PublicGeometricEstimate {
    closest_approach_nautical_miles: f64,
    proximity_margin_nautical_miles: f64,
    geometry_resolution_nautical_miles: f64,
    disclaimer: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct PublicSourceTimes {
    flight_observed_at: Option<DateTime<Utc>>,
    hazard_issued_at: Option<DateTime<Utc>>,
    evaluated_at: DateTime<Utc>,
}

pub fn public_attention_router() -> Router {
    Router::new().route("/api/public/replay/attention", get(public_attention))
}

async fn public_attention() -> impl IntoResponse {
    match &*PUBLIC_PICTURE {
        Ok(picture) => (StatusCode::OK, Json(picture.clone())).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "code": "public_replay_unavailable",
                    "message": "The deterministic replay explanation is unavailable"
                }
            })),
        )
            .into_response(),
    }
}

fn build_public_attention_picture() -> Result<PublicAttentionPicture, String> {
    let scenario =
        ReplayScenario::from_json(PORTFOLIO_SCENARIO).map_err(|error| error.to_string())?;
    let evaluated_at = scenario.start_time + Duration::seconds(60);
    let route = canonical_route(&scenario)?;
    let hazard = canonical_hazard(&scenario)?;
    let altitude = latest_altitude(&scenario, route.flight_id, evaluated_at)?;
    let route_altitude_band = altitude.map(|value| AltitudeBand {
        lower: Some(value),
        upper: Some(value),
    });
    let decision = RouteHazardRule::default()
        .evaluate(RouteHazardInput {
            route: &route,
            hazard: &hazard,
            evaluated_at,
            route_altitude_band: route_altitude_band.as_ref(),
            progress: None,
        })
        .map_err(|error| error.to_string())?;
    let candidate = candidate_from_route_hazard(&route, &hazard, decision).ok_or_else(|| {
        "portfolio attention flight did not produce an alert candidate".to_string()
    })?;

    let aircraft = scenario
        .flights
        .iter()
        .map(|flight| {
            let observed_at = latest_observed_at(&scenario, flight.id, evaluated_at);
            if flight.callsign == ATTENTION_CALLSIGN {
                let evidence = &candidate.decision.evidence;
                PublicAircraftAttention {
                    callsign: flight.callsign.clone(),
                    state: PublicAttentionState::RequiresAttention,
                    priority: Some(candidate.severity),
                    summary: "A significant convective hazard intersects the remaining replay route at the aircraft's demonstrated altitude.".into(),
                    observed_facts: vec![
                        PublicAttentionFact {
                            label: "Replay route",
                            value: format!(
                                "{} to {} · route version {}",
                                flight.origin_airport_code,
                                flight.destination_airport_code,
                                route.route_version
                            ),
                        },
                        PublicAttentionFact {
                            label: "Aircraft altitude",
                            value: altitude.map(format_altitude).unwrap_or_else(|| "Unavailable".into()),
                        },
                        PublicAttentionFact {
                            label: "Hazard evidence",
                            value: format!(
                                "{} · {:?} · revision {}",
                                hazard.hazard_type, hazard.severity, hazard.revision
                            ).to_lowercase(),
                        },
                    ],
                    score: Some(candidate.attention.clone()),
                    rule_result: Some(PublicRuleResult {
                        rule_id: evidence.rule_id.clone(),
                        rule_version: evidence.rule_version,
                        outcome: "match",
                        route_version: evidence.route_version,
                        hazard_revision: evidence.hazard_revision,
                        horizontal_relation: horizontal_relation(evidence.horizontal_relation),
                        altitude_relation: altitude_relation(evidence.altitude_relation),
                    }),
                    geometric_estimate: Some(PublicGeometricEstimate {
                        closest_approach_nautical_miles: evidence.closest_approach_nm,
                        proximity_margin_nautical_miles: evidence.proximity_margin_nm,
                        geometry_resolution_nautical_miles: evidence.geometry_resolution_nm,
                        disclaimer: "Geometric rule estimate, not a filed route, clearance, destination prediction, or provider observation.",
                    }),
                    source_times: PublicSourceTimes {
                        flight_observed_at: observed_at,
                        hazard_issued_at: Some(hazard.issued_at),
                        evaluated_at,
                    },
                }
            } else {
                PublicAircraftAttention {
                    callsign: flight.callsign.clone(),
                    state: PublicAttentionState::NotEvaluated,
                    priority: None,
                    summary: "Not evaluated: this replay aircraft has no route evidence in the current scenario frame.".into(),
                    observed_facts: vec![PublicAttentionFact {
                        label: "Replay position",
                        value: observed_at
                            .map(|value| value.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "Unavailable".into()),
                    }],
                    score: None,
                    rule_result: None,
                    geometric_estimate: None,
                    source_times: PublicSourceTimes {
                        flight_observed_at: observed_at,
                        hazard_issued_at: None,
                        evaluated_at,
                    },
                }
            }
        })
        .collect();

    Ok(PublicAttentionPicture {
        schema_version: 1,
        scenario_id: scenario.id,
        scenario_time: evaluated_at,
        source: "portfolio deterministic replay",
        aircraft,
    })
}

fn canonical_route(scenario: &ReplayScenario) -> Result<PlannedRoute, String> {
    canonical_events(scenario)?
        .find_map(|event| match event {
            CanonicalEvent::PlannedRoute(route) => Some(route),
            _ => None,
        })
        .ok_or_else(|| "portfolio replay route is missing".into())
}

fn canonical_hazard(scenario: &ReplayScenario) -> Result<WeatherHazard, String> {
    canonical_events(scenario)?
        .find_map(|event| match event {
            CanonicalEvent::WeatherHazard(hazard) => Some(hazard),
            _ => None,
        })
        .ok_or_else(|| "portfolio replay hazard is missing".into())
}

fn canonical_events(
    scenario: &ReplayScenario,
) -> Result<impl Iterator<Item = CanonicalEvent> + '_, String> {
    let batches = scenario
        .events
        .iter()
        .map(|event| scenario.batch_for(event).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(batches.into_iter().flat_map(|batch| batch.events))
}

fn latest_altitude(
    scenario: &ReplayScenario,
    flight_id: FlightId,
    evaluated_at: DateTime<Utc>,
) -> Result<Option<Altitude>, String> {
    let mut latest = None;
    for event in &scenario.events {
        let event_time = scenario.start_time + Duration::milliseconds(event.offset_ms as i64);
        if event_time > evaluated_at {
            continue;
        }
        if let ScenarioPayload::Position {
            flight_id: event_flight_id,
            altitude,
            ..
        } = &event.payload
            && *event_flight_id == flight_id
        {
            latest = *altitude;
        }
    }
    Ok(latest)
}

fn latest_observed_at(
    scenario: &ReplayScenario,
    flight_id: FlightId,
    evaluated_at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    scenario.events.iter().rev().find_map(|event| {
        let event_time = scenario.start_time + Duration::milliseconds(event.offset_ms as i64);
        (event_time <= evaluated_at
            && matches!(
                event.payload,
                ScenarioPayload::Position { flight_id: candidate, .. } if candidate == flight_id
            ))
        .then_some(event_time)
    })
}

fn format_altitude(altitude: Altitude) -> String {
    format!("{} {:?}", altitude.value, altitude.unit).to_lowercase()
}

fn horizontal_relation(value: crate::alerting::HorizontalRelation) -> &'static str {
    match value {
        crate::alerting::HorizontalRelation::Intersects => "intersects",
        crate::alerting::HorizontalRelation::WithinMargin => "within_margin",
        crate::alerting::HorizontalRelation::Clear => "clear",
        crate::alerting::HorizontalRelation::BehindRouteProgress => "behind_route_progress",
    }
}

fn altitude_relation(value: crate::alerting::AltitudeRelation) -> &'static str {
    match value {
        crate::alerting::AltitudeRelation::Overlap => "overlap",
        crate::alerting::AltitudeRelation::Disjoint => "disjoint",
        crate::alerting::AltitudeRelation::Indeterminate => "indeterminate",
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    #[test]
    fn deterministic_picture_has_one_explainable_attention_flight() {
        let picture = build_public_attention_picture().unwrap();
        let attention = picture
            .aircraft
            .iter()
            .find(|aircraft| aircraft.callsign == ATTENTION_CALLSIGN)
            .unwrap();

        assert!(matches!(
            attention.state,
            PublicAttentionState::RequiresAttention
        ));
        assert_eq!(attention.priority, Some(AlertSeverity::Critical));
        assert_eq!(attention.score.as_ref().unwrap().total, 85);
        assert_eq!(attention.rule_result.as_ref().unwrap().outcome, "match");
        assert_eq!(attention.observed_facts.len(), 3);
        assert_eq!(
            picture
                .aircraft
                .iter()
                .filter(|aircraft| matches!(aircraft.state, PublicAttentionState::NotEvaluated))
                .count(),
            2
        );
    }

    #[test]
    fn serialized_picture_excludes_protected_and_raw_fields() {
        let serialized = serde_json::to_string(&build_public_attention_picture().unwrap()).unwrap();
        for forbidden in [
            "operator_id",
            "alert_id",
            "hazard_id",
            "route_id",
            "envelope",
            "raw_payload",
            "dedupe_key",
            "series_key",
            "dispatcher",
            "audit",
        ] {
            assert!(!serialized.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[tokio::test]
    async fn public_route_returns_the_sanitized_contract_without_authentication() {
        let response = public_attention_router()
            .oneshot(
                Request::get("/api/public/replay/attention")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["scenario_id"], "m1-operations-v1");
        assert_eq!(payload["aircraft"].as_array().unwrap().len(), 3);
    }
}
