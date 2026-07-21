use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{TimeZone, Utc};
use flight_tracker_api::{
    build_router,
    domain::OperatorId,
    weather::noaa::{
        NoaaFeed, NoaaPayload, NoaaStore, PersistedNoaaRecord, SourceHealthTracker, prepare_records,
    },
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const REQUIRED_TABLES: &[&str] = &[
    "operators",
    "provider_envelopes",
    "airport_observations",
    "flights",
    "aircraft_positions",
    "planned_routes",
    "weather_hazards",
    "alerts",
    "alert_evidence",
    "alert_actions",
    "source_health",
    "ingestion_failures",
];

/// This contract test is enabled in the integration job by TEST_DATABASE_URL.
/// Unit-only environments skip it because a real PostGIS instance is required.
#[tokio::test]
async fn canonical_schema_migrates_with_spatial_and_tenant_invariants() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL not set; skipping PostGIS schema contract test");
        return;
    };

    let pool = PgPool::connect(&database_url).await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let tables = sqlx::query_scalar::<_, String>(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    for required in REQUIRED_TABLES {
        assert!(
            tables.iter().any(|table| table == required),
            "missing canonical table {required}"
        );
    }

    let geometry_contract = sqlx::query_as::<_, (String, String, i32)>(
        r#"
        SELECT f_table_name, type, srid
        FROM geometry_columns
        WHERE f_table_schema = 'public'
          AND f_table_name IN (
              'aircraft_positions', 'airport_observations', 'planned_routes', 'weather_hazards'
          )
        ORDER BY f_table_name
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        geometry_contract,
        vec![
            ("aircraft_positions".to_owned(), "POINT".to_owned(), 4326),
            ("airport_observations".to_owned(), "POINT".to_owned(), 4326),
            ("planned_routes".to_owned(), "LINESTRING".to_owned(), 4326),
            ("weather_hazards".to_owned(), "POLYGON".to_owned(), 4326),
        ]
    );

    assert_provider_record_revisions_are_allowed_but_identical_retries_are_rejected(&pool).await;
    assert_cross_operator_source_reference_is_rejected(&pool).await;
    assert_noaa_records_are_transactional_idempotent_and_revisioned(&pool).await;
}

async fn assert_provider_record_revisions_are_allowed_but_identical_retries_are_rejected(
    pool: &PgPool,
) {
    let mut transaction = pool.begin().await.unwrap();

    sqlx::query(
        r#"
        INSERT INTO operators (id, code, display_name)
        VALUES ('00000000-0000-0000-0000-000000000101', 'REV', 'Revision Test')
        "#,
    )
    .execute(&mut *transaction)
    .await
    .unwrap();

    for (id, hash) in [
        (
            "00000000-0000-0000-0000-000000000110",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        (
            "00000000-0000-0000-0000-000000000111",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO provider_envelopes (
                id, operator_id, schema_version, provider, feed, provider_record_id,
                received_at, raw_payload_sha256, raw_payload
            ) VALUES (
                $1::uuid, '00000000-0000-0000-0000-000000000101', 1,
                'simulation', 'revision-check', 'shared-record', NOW(), $2, '{}'::jsonb
            )
            "#,
        )
        .bind(id)
        .bind(hash)
        .execute(&mut *transaction)
        .await
        .unwrap();
    }

    let identical_retry = sqlx::query(
        r#"
        INSERT INTO provider_envelopes (
            id, operator_id, schema_version, provider, feed, provider_record_id,
            received_at, raw_payload_sha256, raw_payload
        ) VALUES (
            '00000000-0000-0000-0000-000000000112',
            '00000000-0000-0000-0000-000000000101', 1,
            'simulation', 'revision-check', 'shared-record', NOW(),
            'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            '{}'::jsonb
        )
        "#,
    )
    .execute(&mut *transaction)
    .await;

    assert!(
        identical_retry.is_err(),
        "identical provider record retries must be idempotent"
    );

    transaction.rollback().await.unwrap();
}

async fn assert_cross_operator_source_reference_is_rejected(pool: &PgPool) {
    let mut transaction = pool.begin().await.unwrap();

    sqlx::query(
        r#"
        INSERT INTO operators (id, code, display_name) VALUES
            ('00000000-0000-0000-0000-000000000001', 'ONE', 'Operator One'),
            ('00000000-0000-0000-0000-000000000002', 'TWO', 'Operator Two')
        "#,
    )
    .execute(&mut *transaction)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO provider_envelopes (
            id, operator_id, schema_version, provider, feed, received_at,
            raw_payload_sha256, raw_payload
        ) VALUES (
            '00000000-0000-0000-0000-000000000010',
            '00000000-0000-0000-0000-000000000001',
            1, 'simulation', 'tenant-check', NOW(),
            'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            '{}'::jsonb
        )
        "#,
    )
    .execute(&mut *transaction)
    .await
    .unwrap();

    let cross_operator_insert = sqlx::query(
        r#"
        INSERT INTO flights (
            id, operator_id, source_envelope_id, schema_version, event_time,
            received_at, processed_at, status
        ) VALUES (
            '00000000-0000-0000-0000-000000000020',
            '00000000-0000-0000-0000-000000000002',
            '00000000-0000-0000-0000-000000000010',
            1, NOW(), NOW(), NOW(), 'active'
        )
        "#,
    )
    .execute(&mut *transaction)
    .await;

    assert!(
        cross_operator_insert.is_err(),
        "database must reject a source envelope owned by another operator"
    );

    transaction.rollback().await.unwrap();
}

async fn assert_noaa_records_are_transactional_idempotent_and_revisioned(pool: &PgPool) {
    let operator_uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000201").unwrap();
    cleanup_noaa_test_records(pool, operator_uuid).await;
    sqlx::query("INSERT INTO operators (id, code, display_name) VALUES ($1, 'NOAA', 'NOAA Test')")
        .bind(operator_uuid)
        .execute(pool)
        .await
        .unwrap();
    let operator_id = OperatorId::from_uuid(operator_uuid);
    let received_at = Utc.with_ymd_and_hms(2026, 7, 21, 5, 20, 0).unwrap();
    let store = NoaaStore::new(pool.clone());

    let metar = records(
        NoaaFeed::Metar,
        include_str!("../../../fixtures/noaa/metar-normal.json"),
        operator_id,
        received_at,
    )
    .remove(0);
    assert!(matches!(
        store.persist_record(metar.clone()).await.unwrap(),
        PersistedNoaaRecord::Applied(_)
    ));
    assert!(matches!(
        store.persist_record(metar).await.unwrap(),
        PersistedNoaaRecord::Duplicate
    ));

    for fixture in [
        include_str!("../../../fixtures/noaa/airsigmet-normal.json"),
        include_str!("../../../fixtures/noaa/airsigmet-amended.json"),
        include_str!("../../../fixtures/noaa/airsigmet-cancelled.json"),
    ] {
        let record = records(NoaaFeed::AirSigmet, fixture, operator_id, received_at).remove(0);
        assert!(matches!(
            store.persist_record(record).await.unwrap(),
            PersistedNoaaRecord::Applied(_)
        ));
    }

    let malformed = records(
        NoaaFeed::Metar,
        include_str!("../../../fixtures/noaa/metar-malformed.json"),
        operator_id,
        received_at,
    )
    .remove(0);
    assert!(matches!(
        store.persist_record(malformed).await.unwrap(),
        PersistedNoaaRecord::Quarantined { .. }
    ));

    let airport_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM airport_observations WHERE operator_id = $1",
    )
    .bind(operator_uuid)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        airport_count, 1,
        "duplicate METAR must not create another fact"
    );

    let revisions = sqlx::query_as::<_, (i32, String, bool)>(
        r#"
        SELECT revision, status, supersedes_id IS NOT NULL
        FROM weather_hazards
        WHERE operator_id = $1
        ORDER BY revision
        "#,
    )
    .bind(operator_uuid)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        revisions,
        vec![
            (1, "active".into(), false),
            (2, "active".into(), true),
            (3, "cancelled".into(), true),
        ]
    );

    let quarantined = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE pe.processed_at IS NULL),
            COUNT(ifail.id)
        FROM provider_envelopes pe
        LEFT JOIN ingestion_failures ifail
          ON ifail.operator_id = pe.operator_id
         AND ifail.source_envelope_id = pe.id
        WHERE pe.operator_id = $1
        "#,
    )
    .bind(operator_uuid)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(quarantined, (1, 1));

    let mut health = SourceHealthTracker::new(
        operator_id,
        NoaaFeed::Metar,
        std::time::Duration::from_secs(900),
    );
    store
        .upsert_source_health(&health.success(received_at, Some(received_at)))
        .await
        .unwrap();
    let response = build_router(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/api/source-health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["data"][0]["feed"], "metar");
    assert_eq!(body["data"][0]["state"], "healthy");
    cleanup_noaa_test_records(pool, operator_uuid).await;
}

async fn cleanup_noaa_test_records(pool: &PgPool, operator_id: Uuid) {
    for table in [
        "ingestion_failures",
        "airport_observations",
        "weather_hazards",
        "source_health",
        "provider_envelopes",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE operator_id = $1"))
            .bind(operator_id)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM operators WHERE id = $1")
        .bind(operator_id)
        .execute(pool)
        .await
        .unwrap();
}

fn records(
    feed: NoaaFeed,
    fixture: &str,
    operator_id: OperatorId,
    received_at: chrono::DateTime<Utc>,
) -> Vec<flight_tracker_api::weather::noaa::PreparedNoaaRecord> {
    prepare_records(
        NoaaPayload {
            feed,
            value: Some(serde_json::from_str::<Value>(fixture).unwrap()),
        },
        operator_id,
        received_at,
    )
}
