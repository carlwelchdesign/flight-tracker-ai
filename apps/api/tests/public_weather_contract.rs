use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use flight_tracker_api::{domain::OperatorId, weather::public_weather_router};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// Exercises the public contract against the real PostGIS schema when available.
/// Unit-only environments skip instead of replacing geospatial persistence with a mock.
#[tokio::test]
async fn populated_public_weather_is_noaa_only_and_sanitized() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL not set; skipping FT-408 public weather contract");
        return;
    };
    let pool = PgPool::connect(&database_url).await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let operator_id = OperatorId::new();
    let envelope_id = Uuid::new_v4();
    let observation_id = Uuid::new_v4();
    let source_health_id = Uuid::new_v4();
    seed_noaa_observation(
        &pool,
        operator_id,
        envelope_id,
        observation_id,
        source_health_id,
    )
    .await;

    let response = public_weather_router(pool.clone(), Some(operator_id))
        .oneshot(
            Request::builder()
                .uri("/api/public/weather")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["state"], "current");
    assert_eq!(payload["observations"][0]["station_code"], "KSFO");
    assert_eq!(payload["observations"][0]["source"]["provider"], "noaa-awc");
    assert_eq!(
        payload["observations"][0]["point"]["longitude_degrees"],
        -122.375
    );
    assert_eq!(payload["sources"][0]["state"], "healthy");
    let serialized = String::from_utf8(body.to_vec()).unwrap();
    for protected_field in [
        "operator_id",
        "source_envelope_id",
        "raw_text",
        "raw_payload",
    ] {
        assert!(!serialized.contains(protected_field));
    }
    assert!(!serialized.contains("SENSITIVE RAW METAR"));
    assert!(!serialized.contains(&operator_id.as_uuid().to_string()));

    cleanup(&pool, operator_id).await;
}

async fn seed_noaa_observation(
    pool: &PgPool,
    operator_id: OperatorId,
    envelope_id: Uuid,
    observation_id: Uuid,
    source_health_id: Uuid,
) {
    sqlx::query("INSERT INTO operators (id, code, display_name) VALUES ($1, $2, $3)")
        .bind(operator_id.as_uuid())
        .bind(format!("ft408-{}", Uuid::new_v4()))
        .bind("FT-408 contract")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        INSERT INTO provider_envelopes (
            id, operator_id, schema_version, provider, feed, provider_record_id,
            event_time, received_at, processed_at, raw_payload_sha256, raw_payload
        ) VALUES ($1, $2, 1, 'noaa-awc', 'metar', 'KSFO-contract',
            NOW(), NOW(), NOW(), $3, $4)
        "#,
    )
    .bind(envelope_id)
    .bind(operator_id.as_uuid())
    .bind("0".repeat(64))
    .bind(json!({ "raw": "SENSITIVE RAW METAR" }))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO airport_observations (
            id, operator_id, source_envelope_id, schema_version, event_time,
            received_at, processed_at, station_code, report_type, raw_text,
            provider_received_at, position, wind_direction_true_degrees,
            wind_speed_knots, wind_gust_knots, visibility_statute_miles,
            visibility_greater_than, ceiling_feet_agl, flight_category
        ) VALUES ($1, $2, $3, 1, NOW(), NOW(), NOW(), 'KSFO', 'METAR',
            'SENSITIVE RAW METAR', NOW(), ST_SetSRID(ST_MakePoint(-122.375, 37.619), 4326),
            280, 18, 27, 10, TRUE, 2200, 'marginal_visual')
        "#,
    )
    .bind(observation_id)
    .bind(operator_id.as_uuid())
    .bind(envelope_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO source_health (
            id, operator_id, schema_version, provider, feed, state, observed_at,
            last_attempt_at, last_success_at, newest_event_at, consecutive_failures,
            delay_seconds, stale_after_seconds, last_error_code
        ) VALUES ($1, $2, 1, 'noaa-awc', 'metar', 'healthy', NOW(), NOW(), NOW(), NOW(), 0, 0, 900, NULL)
        "#,
    )
    .bind(source_health_id)
    .bind(operator_id.as_uuid())
    .execute(pool)
    .await
    .unwrap();
}

async fn cleanup(pool: &PgPool, operator_id: OperatorId) {
    sqlx::query("DELETE FROM airport_observations WHERE operator_id = $1")
        .bind(operator_id.as_uuid())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM source_health WHERE operator_id = $1")
        .bind(operator_id.as_uuid())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM provider_envelopes WHERE operator_id = $1")
        .bind(operator_id.as_uuid())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM operators WHERE id = $1")
        .bind(operator_id.as_uuid())
        .execute(pool)
        .await
        .unwrap();
}
