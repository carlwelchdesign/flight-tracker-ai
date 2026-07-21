use chrono::Duration;
use flight_tracker_api::{
    alerting::{
        AltitudeRelation, HazardTemporalState, HorizontalRelation, ROUTE_HAZARD_RULE_ID,
        ROUTE_HAZARD_RULE_VERSION, RouteHazardDecision, RouteHazardInput, RouteHazardOutcome,
        RouteHazardRule, RouteHazardRuleConfig, RouteProgress, RouteTemporalState,
    },
    domain::{
        AltitudeBand, CanonicalEvent, GeoPoint, PlannedRoute, WeatherHazard, WeatherHazardStatus,
    },
    replay::ReplayScenario,
};
use serde::Deserialize;

const SCENARIO_JSON: &str = include_str!("../../../fixtures/replay/m2-route-hazard-v1.json");
const GOLDEN_JSON: &str = include_str!("../../../fixtures/rules/route-hazard-golden-v1.json");

#[derive(Debug, Deserialize)]
struct GoldenFixture {
    schema_version: u16,
    rule_id: String,
    rule_version: u32,
    source_scenario: String,
    config: RouteHazardRuleConfig,
    review: FixtureReview,
    cases: Vec<GoldenCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureReview {
    status: String,
    reviewer: Option<String>,
    reviewed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoldenCase {
    id: String,
    rationale: String,
    route_latitude_offset_degrees: f64,
    evaluated_at_offset_ms: i64,
    route_altitude_band: Option<AltitudeBand>,
    hazard_status: Option<WeatherHazardStatus>,
    progress: Option<ProgressFixture>,
    expected: ExpectedDecision,
}

#[derive(Debug, Deserialize)]
struct ProgressFixture {
    segment_index: usize,
    segment_fraction: f64,
}

#[derive(Debug, Deserialize)]
struct ExpectedDecision {
    outcome: RouteHazardOutcome,
    horizontal_relation: HorizontalRelation,
    altitude_relation: AltitudeRelation,
    route_temporal_state: RouteTemporalState,
    hazard_temporal_state: HazardTemporalState,
    closest_approach_nm_min: f64,
    closest_approach_nm_max: f64,
}

#[test]
fn golden_cases_cover_geometry_time_altitude_and_direction() {
    let fixture = fixture();
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.rule_id, ROUTE_HAZARD_RULE_ID);
    assert_eq!(fixture.rule_version, ROUTE_HAZARD_RULE_VERSION);
    assert_eq!(
        fixture.source_scenario,
        "fixtures/replay/m2-route-hazard-v1.json"
    );
    assert!(matches!(
        fixture.review.status.as_str(),
        "awaiting_postgis_oracle" | "verified_by_postgis_oracle"
    ));
    assert_eq!(
        fixture.review.reviewer.as_deref(),
        Some("postgis-3.5-cross-engine-review")
    );
    assert_eq!(
        fixture.review.reviewed_at.is_some(),
        fixture.review.status == "verified_by_postgis_oracle"
    );

    let scenario = scenario();
    let rule = RouteHazardRule::new(
        RouteHazardRuleConfig::new(
            fixture.config.proximity_margin_nm,
            fixture.config.geometry_resolution_nm,
        )
        .unwrap(),
    )
    .unwrap();
    let (base_route, hazard) = route_and_hazard(&scenario);

    for case in &fixture.cases {
        assert!(
            !case.rationale.trim().is_empty(),
            "{} lacks rationale",
            case.id
        );
        let decision = evaluate_case(&rule, &scenario, &base_route, &hazard, case);
        assert_eq!(
            decision.outcome, case.expected.outcome,
            "{} outcome",
            case.id
        );
        assert_eq!(
            decision.evidence.horizontal_relation, case.expected.horizontal_relation,
            "{} horizontal relation",
            case.id
        );
        assert_eq!(
            decision.evidence.altitude_relation, case.expected.altitude_relation,
            "{} altitude relation",
            case.id
        );
        assert_eq!(
            decision.evidence.temporal_relation.route, case.expected.route_temporal_state,
            "{} route time",
            case.id
        );
        assert_eq!(
            decision.evidence.temporal_relation.hazard, case.expected.hazard_temporal_state,
            "{} hazard time",
            case.id
        );
        assert!(
            (case.expected.closest_approach_nm_min..=case.expected.closest_approach_nm_max)
                .contains(&decision.evidence.closest_approach_nm),
            "{} closest approach {} NM outside [{}, {}]",
            case.id,
            decision.evidence.closest_approach_nm,
            case.expected.closest_approach_nm_min,
            case.expected.closest_approach_nm_max
        );
        assert_eq!(decision.evidence.route_id, base_route.id);
        assert_eq!(decision.evidence.route_version, 7);
        assert_eq!(decision.evidence.hazard_id, hazard.id);
        assert_eq!(decision.evidence.hazard_revision, 1);
        assert_eq!(decision.evidence.rule_id, ROUTE_HAZARD_RULE_ID);
        assert_eq!(decision.evidence.rule_version, ROUTE_HAZARD_RULE_VERSION);
    }
}

#[test]
fn replay_reload_produces_byte_identical_rule_decisions() {
    let fixture_a = fixture();
    let fixture_b = fixture();
    let scenario_a = scenario();
    let scenario_b = scenario();
    let rule = RouteHazardRule::new(fixture_a.config).unwrap();
    let (route_a, hazard_a) = route_and_hazard(&scenario_a);
    let (route_b, hazard_b) = route_and_hazard(&scenario_b);

    for (event_a, event_b) in scenario_a.events.iter().zip(&scenario_b.events) {
        assert_eq!(
            scenario_a.batch_for(event_a).unwrap(),
            scenario_b.batch_for(event_b).unwrap()
        );
    }

    let decisions_a = fixture_a
        .cases
        .iter()
        .map(|case| evaluate_case(&rule, &scenario_a, &route_a, &hazard_a, case))
        .collect::<Vec<_>>();
    let decisions_b = fixture_b
        .cases
        .iter()
        .map(|case| evaluate_case(&rule, &scenario_b, &route_b, &hazard_b, case))
        .collect::<Vec<_>>();

    assert_eq!(
        serde_json::to_vec(&decisions_a).unwrap(),
        serde_json::to_vec(&decisions_b).unwrap()
    );
}

fn fixture() -> GoldenFixture {
    serde_json::from_str(GOLDEN_JSON).unwrap()
}

fn scenario() -> ReplayScenario {
    ReplayScenario::from_json(SCENARIO_JSON).unwrap()
}

fn route_and_hazard(scenario: &ReplayScenario) -> (PlannedRoute, WeatherHazard) {
    let mut route = None;
    let mut hazard = None;
    for event in &scenario.events {
        let batch = scenario.batch_for(event).unwrap();
        match batch.events.into_iter().next().unwrap() {
            CanonicalEvent::PlannedRoute(value) => route = Some(value),
            CanonicalEvent::WeatherHazard(value) => hazard = Some(value),
            _ => {}
        }
    }
    (route.unwrap(), hazard.unwrap())
}

fn evaluate_case(
    rule: &RouteHazardRule,
    scenario: &ReplayScenario,
    base_route: &PlannedRoute,
    base_hazard: &WeatherHazard,
    case: &GoldenCase,
) -> RouteHazardDecision {
    let mut route = base_route.clone();
    let mut hazard = base_hazard.clone();
    if let Some(status) = case.hazard_status {
        hazard.status = status;
    }
    route.path.coordinates = route
        .path
        .coordinates
        .iter()
        .map(|point| {
            let [longitude, latitude] = point.as_geojson_position();
            GeoPoint::new(longitude, latitude + case.route_latitude_offset_degrees).unwrap()
        })
        .collect();
    let progress = case.progress.as_ref().map(|progress| {
        RouteProgress::new(progress.segment_index, progress.segment_fraction).unwrap()
    });
    rule.evaluate(RouteHazardInput {
        route: &route,
        hazard: &hazard,
        evaluated_at: scenario.start_time + Duration::milliseconds(case.evaluated_at_offset_ms),
        route_altitude_band: case.route_altitude_band.as_ref(),
        progress,
    })
    .unwrap()
}
