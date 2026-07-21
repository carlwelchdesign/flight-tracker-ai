use chrono::Duration;
use flight_tracker_api::{
    domain::{CanonicalEvent, PlannedRoute, WeatherHazard},
    replay::ReplayScenario,
};
use serde_json::{Value, json};
use sqlx::PgPool;

const SCENARIO_JSON: &str = include_str!("../../../fixtures/replay/m2-route-hazard-v1.json");
const GOLDEN_JSON: &str = include_str!("../../../fixtures/rules/route-hazard-golden-v1.json");

/// Cross-checks the golden outcomes with PostGIS rather than the Rust rule's
/// geometry implementation. This is the independent fixture oracle required by
/// FT-203 and runs in the dedicated PostGIS CI job.
#[tokio::test]
async fn postgis_independently_confirms_golden_fixture_outcomes() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL not set; skipping PostGIS fixture review");
        return;
    };
    let pool = PgPool::connect(&database_url).await.unwrap();
    let scenario = ReplayScenario::from_json(SCENARIO_JSON).unwrap();
    let (route, hazard) = route_and_hazard(&scenario);
    let fixture: Value = serde_json::from_str(GOLDEN_JSON).unwrap();
    assert_eq!(
        fixture["review"]["reviewer"],
        "postgis-3.5-cross-engine-review"
    );

    for case in fixture["cases"].as_array().unwrap() {
        verify_case(&pool, &scenario, &route, &hazard, case).await;
    }
}

async fn verify_case(
    pool: &PgPool,
    scenario: &ReplayScenario,
    route: &PlannedRoute,
    hazard: &WeatherHazard,
    case: &Value,
) {
    let case_id = case["id"].as_str().unwrap();
    let latitude_offset = case["route_latitude_offset_degrees"].as_f64().unwrap();
    let route_geojson = json!({
        "type": "LineString",
        "coordinates": route.path.coordinates.iter().map(|point| {
            let [longitude, latitude] = point.as_geojson_position();
            [longitude, latitude + latitude_offset]
        }).collect::<Vec<_>>()
    })
    .to_string();
    let hazard_geojson = json!({
        "type": "Polygon",
        "coordinates": [hazard.footprint.exterior.iter().map(|point| {
            point.as_geojson_position()
        }).collect::<Vec<_>>()]
    })
    .to_string();
    let progress = case["progress"]["segment_fraction"].as_f64();
    let evaluated_at = scenario.start_time
        + Duration::milliseconds(case["evaluated_at_offset_ms"].as_i64().unwrap());
    let hazard_status = case["hazard_status"].as_str().unwrap_or("active");
    let route_band = case["route_altitude_band"].as_object();
    let route_lower = route_band.and_then(|band| band["lower"]["value"].as_f64());
    let route_upper = route_band.and_then(|band| band["upper"]["value"].as_f64());
    let hazard_band = hazard.altitude_band.as_ref().unwrap();
    let hazard_lower = f64::from(hazard_band.lower.unwrap().value);
    let hazard_upper = f64::from(hazard_band.upper.unwrap().value);

    let result = sqlx::query_as::<_, (bool, f64, bool, bool, String, String, String)>(
        r#"
        WITH geometry AS (
            SELECT ST_SetSRID(ST_GeomFromGeoJSON($1), 4326) AS full_route,
                   ST_SetSRID(ST_GeomFromGeoJSON($2), 4326) AS hazard
        ), remaining AS (
            SELECT full_route, hazard,
                   CASE WHEN $3::double precision IS NULL THEN full_route
                        ELSE ST_LineSubstring(full_route, $3, 1.0)
                   END AS remaining_route
            FROM geometry
        )
        SELECT ST_Intersects(remaining_route, hazard),
               ST_Distance(remaining_route::geography, hazard::geography) / 1852.0,
               ST_DWithin(remaining_route::geography, hazard::geography, $4),
               ST_DWithin(full_route::geography, hazard::geography, $4),
               CASE WHEN $5 < $6 THEN 'not_yet_effective'
                    WHEN $7::timestamptz IS NOT NULL AND $5 >= $7 THEN 'expired'
                    ELSE 'active' END,
               CASE WHEN $15 = 'cancelled' THEN 'cancelled'
                    WHEN $5 < $8 THEN 'not_yet_valid'
                    WHEN $5 > $9 THEN 'expired'
                    ELSE 'active' END,
               CASE WHEN NOT $10 THEN 'indeterminate'
                    WHEN $11 <= $14 AND $13 <= $12 THEN 'overlap'
                    ELSE 'disjoint' END
        FROM remaining
        "#,
    )
    .bind(route_geojson)
    .bind(hazard_geojson)
    .bind(progress)
    .bind(25.0 * 1_852.0)
    .bind(evaluated_at)
    .bind(route.effective_from)
    .bind(route.effective_to)
    .bind(hazard.valid_from)
    .bind(hazard.valid_to)
    .bind(route_band.is_some())
    .bind(route_lower.unwrap_or(0.0))
    .bind(route_upper.unwrap_or(0.0))
    .bind(hazard_lower)
    .bind(hazard_upper)
    .bind(hazard_status)
    .fetch_one(pool)
    .await
    .unwrap();

    let (
        intersects,
        closest_nm,
        within_margin,
        full_route_within,
        route_time,
        hazard_time,
        altitude,
    ) = result;
    let horizontal = if intersects {
        "intersects"
    } else if within_margin {
        "within_margin"
    } else if progress.is_some() && full_route_within {
        "behind_route_progress"
    } else {
        "clear"
    };
    let expected = &case["expected"];
    assert_eq!(
        horizontal, expected["horizontal_relation"],
        "{case_id} horizontal"
    );
    assert_eq!(
        route_time, expected["route_temporal_state"],
        "{case_id} route time"
    );
    assert_eq!(
        hazard_time, expected["hazard_temporal_state"],
        "{case_id} hazard time"
    );
    assert_eq!(
        altitude, expected["altitude_relation"],
        "{case_id} altitude"
    );
    assert!(
        (fixture_number(case, "/expected/closest_approach_nm_min")
            ..=fixture_number(case, "/expected/closest_approach_nm_max"))
            .contains(&closest_nm),
        "{case_id} PostGIS closest approach {closest_nm} NM"
    );

    let temporal_match = route_time == "active" && hazard_time == "active";
    let horizontally_relevant = horizontal == "intersects" || horizontal == "within_margin";
    let oracle_outcome = if !temporal_match || altitude == "disjoint" || !horizontally_relevant {
        "no_match"
    } else if altitude == "indeterminate" {
        "indeterminate"
    } else if horizontally_relevant {
        "match"
    } else {
        "no_match"
    };
    assert_eq!(oracle_outcome, expected["outcome"], "{case_id} outcome");
}

fn fixture_number(case: &Value, pointer: &str) -> f64 {
    case.pointer(pointer).and_then(Value::as_f64).unwrap()
}

fn route_and_hazard(scenario: &ReplayScenario) -> (PlannedRoute, WeatherHazard) {
    let mut route = None;
    let mut hazard = None;
    for event in &scenario.events {
        match scenario.batch_for(event).unwrap().events.remove(0) {
            CanonicalEvent::PlannedRoute(value) => route = Some(value),
            CanonicalEvent::WeatherHazard(value) => hazard = Some(value),
            _ => {}
        }
    }
    (route.unwrap(), hazard.unwrap())
}
