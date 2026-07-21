use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{TimeZone, Utc};
use flight_tracker_api::{
    alerting::{
        AlertActionRequest, AlertStore, CreateAlertResult, RouteHazardInput, RouteHazardRule,
        candidate_from_route_hazard,
    },
    build_router,
    domain::{
        AlertActionKind, Altitude, AltitudeBand, AltitudeReference, AltitudeUnit, CanonicalEvent,
        OperatorId,
    },
    replay::ReplayScenario,
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
    assert_alert_queue_is_ranked_deduplicated_superseded_and_audited(&pool).await;
}

async fn assert_alert_queue_is_ranked_deduplicated_superseded_and_audited(pool: &PgPool) {
    let scenario = ReplayScenario::from_json(include_str!(
        "../../../fixtures/replay/m2-route-hazard-v1.json"
    ))
    .unwrap();
    let batches = scenario
        .events
        .iter()
        .map(|event| scenario.batch_for(event).unwrap())
        .collect::<Vec<_>>();
    let CanonicalEvent::Flight(flight) = &batches[0].events[0] else {
        panic!("fixture must begin with a flight")
    };
    let CanonicalEvent::PlannedRoute(route) = &batches[1].events[0] else {
        panic!("fixture must include a route")
    };
    let CanonicalEvent::WeatherHazard(hazard) = &batches[2].events[0] else {
        panic!("fixture must include a hazard")
    };

    sqlx::query(
        "INSERT INTO operators (id, code, display_name) VALUES ($1, 'ALERT', 'Alert Test') ON CONFLICT (id) DO NOTHING",
    )
    .bind(scenario.operator_id.as_uuid())
    .execute(pool)
    .await
    .unwrap();
    for batch in &batches {
        sqlx::query(
            r#"
            INSERT INTO provider_envelopes (
                id, operator_id, schema_version, provider, feed, provider_record_id,
                event_time, received_at, processed_at, raw_payload_sha256, raw_payload
            ) VALUES ($1,$2,1,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(batch.envelope.id.as_uuid())
        .bind(batch.envelope.operator_id.as_uuid())
        .bind(&batch.envelope.provider)
        .bind(&batch.envelope.feed)
        .bind(&batch.envelope.provider_record_id)
        .bind(batch.envelope.event_time)
        .bind(batch.envelope.received_at)
        .bind(batch.envelope.processed_at)
        .bind(&batch.envelope.raw_payload_sha256)
        .bind(&batch.envelope.raw_payload)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query(
        r#"
        INSERT INTO flights (
            id,operator_id,source_envelope_id,schema_version,event_time,received_at,processed_at,
            callsign,status
        ) VALUES ($1,$2,$3,1,$4,$5,$6,$7,'active')
        ON CONFLICT (operator_id,id) DO NOTHING
        "#,
    )
    .bind(flight.id.as_uuid())
    .bind(flight.operator_id.as_uuid())
    .bind(flight.source.envelope_id.as_uuid())
    .bind(flight.times.event_time)
    .bind(flight.times.received_at)
    .bind(flight.times.processed_at)
    .bind(&flight.callsign)
    .execute(pool)
    .await
    .unwrap();
    let footprint = serde_json::json!({
        "type":"Polygon",
        "coordinates":[hazard.footprint.exterior.iter().map(|point| point.as_geojson_position()).collect::<Vec<_>>()]
    });
    sqlx::query(
        r#"
        INSERT INTO weather_hazards (
            id,operator_id,source_envelope_id,schema_version,event_time,received_at,processed_at,
            hazard_type,severity,valid_from,valid_to,footprint,external_series_id,revision,
            status,issued_at
        ) VALUES ($1,$2,$3,1,$4,$5,$6,$7,'significant',$8,$9,
            ST_SetSRID(ST_GeomFromGeoJSON($10),4326),$11,1,'active',$4)
        ON CONFLICT (operator_id,id) DO NOTHING
        "#,
    )
    .bind(hazard.id.as_uuid())
    .bind(hazard.operator_id.as_uuid())
    .bind(hazard.source.envelope_id.as_uuid())
    .bind(hazard.times.event_time)
    .bind(hazard.times.received_at)
    .bind(hazard.times.processed_at)
    .bind(&hazard.hazard_type)
    .bind(hazard.valid_from)
    .bind(hazard.valid_to)
    .bind(footprint.to_string())
    .bind(&hazard.external_series_id)
    .execute(pool)
    .await
    .unwrap();

    let altitude = Altitude {
        value: 20_000,
        unit: AltitudeUnit::Feet,
        reference: AltitudeReference::MeanSeaLevel,
    };
    let altitude_band = AltitudeBand {
        lower: Some(altitude),
        upper: Some(altitude),
    };
    let evaluated_at = scenario.start_time + chrono::Duration::minutes(1);
    let decision = RouteHazardRule::default()
        .evaluate(RouteHazardInput {
            route,
            hazard,
            evaluated_at,
            route_altitude_band: Some(&altitude_band),
            progress: None,
        })
        .unwrap();
    let candidate = candidate_from_route_hazard(route, hazard, decision).unwrap();
    let store = AlertStore::new(pool.clone());
    let created_id = match store
        .create_from_candidate(&candidate, evaluated_at)
        .await
        .unwrap()
    {
        CreateAlertResult::Created(id) => id,
        CreateAlertResult::Duplicate(_) => panic!("first candidate must create an alert"),
    };
    assert_eq!(
        store
            .create_from_candidate(&candidate, evaluated_at)
            .await
            .unwrap(),
        CreateAlertResult::Duplicate(created_id)
    );

    let low_priority_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO alerts (
            id,operator_id,schema_version,event_time,received_at,processed_at,alert_type,severity,
            lifecycle,rule_id,rule_version,dedupe_key,series_key,alert_revision,
            attention_score,score_version,evidence
        ) VALUES ($1,$2,1,$3,$3,$3,'test_information','information','open','test_rule',1,$4,$4,1,5,1,'{}')
        "#,
    )
    .bind(low_priority_id)
    .bind(scenario.operator_id.as_uuid())
    .bind(evaluated_at)
    .bind(format!("low-priority-{low_priority_id}"))
    .execute(pool)
    .await
    .unwrap();
    let ranked = store.list_queue(scenario.operator_id, false).await.unwrap();
    assert_eq!(ranked[0].id, created_id);
    assert_eq!(ranked[1].id, low_priority_id);

    let mut revised_route = route.clone();
    revised_route.route_version += 1;
    let revised_decision = RouteHazardRule::default()
        .evaluate(RouteHazardInput {
            route: &revised_route,
            hazard,
            evaluated_at,
            route_altitude_band: Some(&altitude_band),
            progress: None,
        })
        .unwrap();
    let revised_candidate = candidate_from_route_hazard(&revised_route, hazard, revised_decision)
        .expect("revised matching evidence creates a candidate");
    let revised_id = match store
        .create_from_candidate(&revised_candidate, evaluated_at)
        .await
        .unwrap()
    {
        CreateAlertResult::Created(id) => id,
        CreateAlertResult::Duplicate(_) => panic!("material route revision must create an alert"),
    };
    let prior = store
        .detail(scenario.operator_id, created_id)
        .await
        .unwrap();
    assert_eq!(prior.alert.lifecycle, "resolved");
    assert_eq!(prior.actions.len(), 1);
    assert_eq!(prior.actions[0].actor_id, "system:alert-supersession");

    let acknowledge = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Acknowledge,
        actor_id: "dispatcher:test".into(),
        idempotency_key: "ack-revised-alert".into(),
        comment: None,
    };
    let acknowledged = store
        .apply_action(revised_id, &acknowledge, evaluated_at)
        .await
        .unwrap();
    assert_eq!(acknowledged.alert.lifecycle, "acknowledged");
    let retried = store
        .apply_action(revised_id, &acknowledge, evaluated_at)
        .await
        .unwrap();
    assert_eq!(retried.actions.len(), 1);

    let comment = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Comment,
        actor_id: "dispatcher:test".into(),
        idempotency_key: "comment-revised-alert".into(),
        comment: Some("Coordinating with the flight crew".into()),
    };
    let commented = store
        .apply_action(revised_id, &comment, evaluated_at)
        .await
        .unwrap();
    assert_eq!(commented.alert.lifecycle, "acknowledged");
    assert_eq!(commented.actions.len(), 2);

    let missing_reason = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Dismiss,
        actor_id: "dispatcher:test".into(),
        idempotency_key: "invalid-dismiss-revised-alert".into(),
        comment: None,
    };
    assert!(
        store
            .apply_action(revised_id, &missing_reason, evaluated_at)
            .await
            .is_err()
    );

    let dismiss = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Dismiss,
        actor_id: "dispatcher:test".into(),
        idempotency_key: "dismiss-revised-alert".into(),
        comment: Some("Duplicate dispatch information".into()),
    };
    let dismissed = store
        .apply_action(revised_id, &dismiss, evaluated_at)
        .await
        .unwrap();
    assert_eq!(dismissed.alert.lifecycle, "dismissed");
    assert_eq!(dismissed.actions.len(), 3);

    let current = store.list_queue(scenario.operator_id, false).await.unwrap();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].id, low_priority_id);
    let history = store.list_queue(scenario.operator_id, true).await.unwrap();
    assert!(history.iter().any(|alert| alert.id == revised_id));
    assert!(!history.iter().any(|alert| alert.id == created_id));
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
    let metar_source_id = metar.envelope.id.as_uuid();
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

    sqlx::query(
        "UPDATE airport_observations SET event_time = NOW() - INTERVAL '5 minutes' WHERE operator_id = $1",
    )
    .bind(operator_uuid)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE weather_hazards SET valid_from = NOW() - INTERVAL '1 hour', valid_to = NOW() + INTERVAL '1 hour' WHERE operator_id = $1",
    )
    .bind(operator_uuid)
    .execute(pool)
    .await
    .unwrap();

    let app = build_router(pool.clone());
    let observations = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/airport-observations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(observations.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&observations.into_body().collect().await.unwrap().to_bytes())
            .unwrap();
    assert_eq!(body["data"][0]["station_code"], "KSFO");
    assert_eq!(body["data"][0]["flight_category"], "visual");
    assert_eq!(body["data"][0]["source"]["provider"], "noaa-awc");

    let hazards = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hazards")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hazards.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&hazards.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["data"][0]["revision"], 3);
    assert_eq!(body["data"][0]["status"], "cancelled");
    assert_eq!(body["data"][0]["severity"], "significant");
    assert_eq!(
        body["data"][0]["footprint"]["exterior"]
            .as_array()
            .unwrap()
            .len(),
        5
    );

    let source = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/source-records/{metar_source_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&source.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["provider"], "noaa-awc");
    assert_eq!(body["raw_payload"]["icaoId"], "KSFO");
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
