use sqlx::PgPool;

const REQUIRED_TABLES: &[&str] = &[
    "operators",
    "provider_envelopes",
    "flights",
    "aircraft_positions",
    "planned_routes",
    "weather_hazards",
    "alerts",
    "alert_evidence",
    "alert_actions",
    "source_health",
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
          AND f_table_name IN ('aircraft_positions', 'planned_routes', 'weather_hazards')
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
            ("planned_routes".to_owned(), "LINESTRING".to_owned(), 4326),
            ("weather_hazards".to_owned(), "POLYGON".to_owned(), 4326),
        ]
    );

    assert_cross_operator_source_reference_is_rejected(&pool).await;
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
