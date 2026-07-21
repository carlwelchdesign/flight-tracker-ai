use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, SecondsFormat, TimeZone, Utc};
use flight_tracker_api::{
    alerting::{
        AlertActionRequest, AlertQueueFilter, AlertStore, AlertStoreError, AssignmentFilter,
        CreateAlertResult, DismissalReason, RouteHazardInput, RouteHazardRule,
        candidate_from_route_hazard,
    },
    auth::{
        AssertionClaims, AssertionConfig, AssertionKey, AuthRole, AuthService, AuthStore,
        DevelopmentIdentity, InternalAssertionVerifier, SessionRevocation,
    },
    build_router,
    domain::{
        AlertActionKind, Altitude, AltitudeBand, AltitudeReference, AltitudeUnit, CanonicalEvent,
        OperatorId,
    },
    replay::ReplayScenario,
    retention::{CreateRetentionPolicy, PreviewRetentionRun, RetentionError, RetentionStore},
    weather::noaa::{
        NoaaFeed, NoaaPayload, NoaaStore, PersistedNoaaRecord, SourceHealthTracker, prepare_records,
    },
};
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
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
    "auth_identities",
    "operator_memberships",
    "auth_session_revocations",
    "authorization_audit_events",
    "retention_policies",
    "retention_runs",
    "data_deletion_tombstones",
];

async fn authenticated_service(pool: &PgPool, operator_id: OperatorId) -> (AuthService, String) {
    authenticated_service_with_role(pool, operator_id, AuthRole::Administrator).await
}

async fn authenticated_service_with_role(
    pool: &PgPool,
    operator_id: OperatorId,
    role: AuthRole,
) -> (AuthService, String) {
    const SECRET: &str = "schema-contract-internal-secret-at-least-32-bytes";
    let store = AuthStore::new(pool.clone());
    store
        .bootstrap_development(&DevelopmentIdentity {
            operator_id,
            operator_code: format!("T{}", &operator_id.as_uuid().simple().to_string()[..6]),
            operator_name: "NOAA Test".into(),
            external_tenant_id: format!("test-{}", operator_id.as_uuid()),
            subject: "schema-contract-admin".into(),
            display_name: "Schema Contract Administrator".into(),
            role,
        })
        .await
        .unwrap();
    let now = Utc::now();
    let claims = AssertionClaims {
        iss: "schema-contract-web".into(),
        aud: "schema-contract-api".into(),
        sub: "schema-contract-admin".into(),
        provider: "development".into(),
        tenant: format!("test-{}", operator_id.as_uuid()),
        sid: "schema-contract-session".into(),
        jti: Uuid::new_v4().to_string(),
        iat: now.timestamp() as u64,
        nbf: now.timestamp() as u64,
        exp: (now + Duration::seconds(60)).timestamp() as u64,
    };
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("schema-contract-primary".into());
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap();
    let verifier = InternalAssertionVerifier::new(AssertionConfig {
        active_key: AssertionKey {
            id: "schema-contract-primary".into(),
            secret: SECRET.into(),
        },
        previous_key: None,
        issuer: "schema-contract-web".into(),
        audience: "schema-contract-api".into(),
        leeway_seconds: 0,
    })
    .unwrap();
    (AuthService::new(verifier, store), format!("Bearer {token}"))
}

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
    assert_audit_review_export_and_monitoring_are_tenant_safe(&pool).await;
    assert_raw_retention_requires_approval_and_suppresses_restore(&pool).await;
    assert_identity_tenant_revocation_and_audit_are_fail_closed(&pool).await;
}

async fn test_auth_context(
    pool: &PgPool,
    operator_id: OperatorId,
    subject: &str,
    role: AuthRole,
) -> flight_tracker_api::auth::AuthContext {
    let store = AuthStore::new(pool.clone());
    let tenant = format!("retention-{}", operator_id.as_uuid());
    store
        .bootstrap_development(&DevelopmentIdentity {
            operator_id,
            operator_code: format!("R{}", &operator_id.as_uuid().simple().to_string()[..6]),
            operator_name: "Retention Test".into(),
            external_tenant_id: tenant.clone(),
            subject: subject.into(),
            display_name: subject.into(),
            role,
        })
        .await
        .unwrap();
    let now = Utc::now();
    store
        .resolve(&AssertionClaims {
            iss: "test-web".into(),
            aud: "test-api".into(),
            sub: subject.into(),
            provider: "development".into(),
            tenant,
            sid: format!("session-{subject}"),
            jti: Uuid::new_v4().to_string(),
            iat: now.timestamp() as u64,
            nbf: now.timestamp() as u64,
            exp: (now + Duration::minutes(5)).timestamp() as u64,
        })
        .await
        .unwrap()
}

async fn assert_raw_retention_requires_approval_and_suppresses_restore(pool: &PgPool) {
    let operator_id = OperatorId::new();
    let other_operator_id = OperatorId::new();
    let requester = test_auth_context(
        pool,
        operator_id,
        &format!("retention-requester-{}", Uuid::new_v4()),
        AuthRole::Administrator,
    )
    .await;
    let approver = test_auth_context(
        pool,
        operator_id,
        &format!("retention-approver-{}", Uuid::new_v4()),
        AuthRole::Administrator,
    )
    .await;
    let other = test_auth_context(
        pool,
        other_operator_id,
        &format!("retention-other-{}", Uuid::new_v4()),
        AuthRole::Administrator,
    )
    .await;
    let now = Utc::now();
    let old_id = Uuid::new_v4();
    let current_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    let old_hash = "c".repeat(64);
    for (id, tenant, received_at, hash, payload) in [
        (
            old_id,
            operator_id,
            now - Duration::hours(2),
            old_hash.clone(),
            serde_json::json!({"secretRaw": "expired"}),
        ),
        (
            current_id,
            operator_id,
            now - Duration::minutes(30),
            "d".repeat(64),
            serde_json::json!({"currentRaw": true}),
        ),
        (
            other_id,
            other_operator_id,
            now - Duration::hours(2),
            "e".repeat(64),
            serde_json::json!({"otherTenantRaw": true}),
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO provider_envelopes (
                id, operator_id, schema_version, provider, feed, provider_record_id,
                received_at, raw_payload_sha256, raw_payload
            ) VALUES ($1,$2,1,'retention-test','positions',$3,$4,$5,$6)
            "#,
        )
        .bind(id)
        .bind(tenant.as_uuid())
        .bind(id.to_string())
        .bind(received_at)
        .bind(hash)
        .bind(payload)
        .execute(pool)
        .await
        .unwrap();
    }

    let store = RetentionStore::new(pool.clone());
    let policy = store
        .create_policy(
            &requester,
            &CreateRetentionPolicy {
                provider: "retention-test".into(),
                retention_seconds: 3_600,
                approval_reference: "legal:FT-401/raw-v1".into(),
            },
            now,
        )
        .await
        .unwrap();
    assert!(matches!(
        store.approve_policy(&requester, policy.id, now).await,
        Err(RetentionError::SeparationOfDuties)
    ));
    let policy = store
        .approve_policy(&approver, policy.id, now)
        .await
        .unwrap();
    assert_eq!(policy.status, "approved");
    assert!(
        store
            .list_policies(other.operator_id)
            .await
            .unwrap()
            .is_empty()
    );

    let run = store
        .preview_run(
            &requester,
            &PreviewRetentionRun {
                policy_id: policy.id,
                evidence_reference: "incident:FT-401/raw-run-1".into(),
            },
            now,
        )
        .await
        .unwrap();
    assert_eq!(run.preview_counts["provider_envelopes"], 1);
    assert!(matches!(
        store.approve_run(&requester, run.id, now).await,
        Err(RetentionError::SeparationOfDuties)
    ));
    store.approve_run(&approver, run.id, now).await.unwrap();
    let completed = store.execute_run(&requester, run.id, now).await.unwrap();
    assert_eq!(completed.status, "completed");
    assert_eq!(completed.deletion_counts.unwrap()["provider_envelopes"], 1);

    let payloads = sqlx::query_as::<_, (Uuid, Value, Option<chrono::DateTime<Utc>>)>(
        r#"
        SELECT id, raw_payload, raw_payload_deleted_at
        FROM provider_envelopes
        WHERE id = ANY($1)
        ORDER BY id
        "#,
    )
    .bind(vec![old_id, current_id, other_id])
    .fetch_all(pool)
    .await
    .unwrap();
    let expired = payloads.iter().find(|row| row.0 == old_id).unwrap();
    assert_eq!(expired.1, serde_json::json!({}));
    assert_eq!(expired.2, Some(now));
    assert_eq!(
        payloads.iter().find(|row| row.0 == current_id).unwrap().1,
        serde_json::json!({"currentRaw": true})
    );
    assert_eq!(
        payloads.iter().find(|row| row.0 == other_id).unwrap().1,
        serde_json::json!({"otherTenantRaw": true})
    );

    sqlx::query("DELETE FROM provider_envelopes WHERE id = $1")
        .bind(old_id)
        .execute(pool)
        .await
        .unwrap();
    let restored_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO provider_envelopes (
            id, operator_id, schema_version, provider, feed, provider_record_id,
            received_at, raw_payload_sha256, raw_payload
        ) VALUES ($1,$2,1,'retention-test','positions',$3,$4,$5,$6)
        "#,
    )
    .bind(restored_id)
    .bind(operator_id.as_uuid())
    .bind(restored_id.to_string())
    .bind(now - Duration::hours(2))
    .bind(&old_hash)
    .bind(serde_json::json!({"secretRaw": "restored"}))
    .execute(pool)
    .await
    .unwrap();
    let restored = sqlx::query_as::<_, (Value, Option<chrono::DateTime<Utc>>)>(
        "SELECT raw_payload, raw_payload_deleted_at FROM provider_envelopes WHERE id = $1",
    )
    .bind(restored_id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(restored, (serde_json::json!({}), Some(now)));

    let completion_audit = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM authorization_audit_events
        WHERE operator_id = $1 AND action = 'retention.run.completed'
          AND target_id = $2
        "#,
    )
    .bind(operator_id.as_uuid())
    .bind(run.id.to_string())
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(completion_audit, 1);
}

async fn assert_audit_review_export_and_monitoring_are_tenant_safe(pool: &PgPool) {
    let operator_id = ReplayScenario::from_json(include_str!(
        "../../../fixtures/replay/m2-route-hazard-v1.json"
    ))
    .unwrap()
    .operator_id;
    let other_operator_id = OperatorId::new();
    let (auth, authorization) = authenticated_service(pool, operator_id).await;
    let actor_identity_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT identity_id FROM operator_memberships WHERE operator_id = $1 AND role = 'administrator' LIMIT 1",
    )
    .bind(operator_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let raw_session_id = format!("sensitive-session-{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO authorization_audit_events (
            id, operator_id, actor_identity_id, action, target_type,
            target_id, occurred_at, metadata
        ) VALUES ($1,$2,$3,'session.revoked','auth_session',$4,NOW(),$5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(operator_id.as_uuid())
    .bind(actor_identity_id)
    .bind(&raw_session_id)
    .bind(serde_json::json!({
        "provider": "development",
        "identity_id": actor_identity_id,
        "reason": "free-form reason must remain private"
    }))
    .execute(pool)
    .await
    .unwrap();

    let (other_auth, _) = authenticated_service(pool, other_operator_id).await;
    let other_identity_id = other_auth
        .store()
        .list_memberships(other_operator_id)
        .await
        .unwrap()[0]
        .identity_id;
    let cross_tenant_marker = format!("cross-tenant-{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO authorization_audit_events (
            id, operator_id, actor_identity_id, action, target_type,
            target_id, occurred_at, metadata
        ) VALUES ($1,$2,$3,'membership.updated','operator_membership',$4,NOW(),'{}')
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(other_operator_id.as_uuid())
    .bind(other_identity_id)
    .bind(&cross_tenant_marker)
    .execute(pool)
    .await
    .unwrap();

    let app = build_router(pool.clone(), auth);
    let review = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit-events?limit=250")
                .header("authorization", &authorization)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(review.status(), StatusCode::OK);
    let review_body = String::from_utf8(
        review
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(review_body.contains("session.revoked"));
    assert!(!review_body.contains(&raw_session_id));
    assert!(!review_body.contains("free-form reason"));
    assert!(!review_body.contains(&cross_tenant_marker));

    let from = (Utc::now() - Duration::hours(1)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let to = (Utc::now() + Duration::minutes(1)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let export = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/admin/audit-events/export?from={from}&to={to}"
                ))
                .header("authorization", &authorization)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export.status(), StatusCode::OK);
    assert_eq!(export.headers()["content-type"], "text/csv; charset=utf-8");
    let export_body = String::from_utf8(
        export
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(export_body.contains("session.revoked"));
    assert!(!export_body.contains(&raw_session_id));
    assert!(!export_body.contains("free-form reason"));
    assert!(!export_body.contains(&cross_tenant_marker));

    let signals = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit-alerts")
                .header("authorization", &authorization)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(signals.status(), StatusCode::OK);
    let signals_body: Value =
        serde_json::from_slice(&signals.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert!(
        signals_body["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|signal| {
                signal["code"] == "high_risk_action"
                    && signal["message"] == "High-risk audit action recorded: session.revoked"
            })
    );

    let (viewer_auth, viewer_authorization) =
        authenticated_service_with_role(pool, OperatorId::new(), AuthRole::Viewer).await;
    let forbidden = build_router(pool.clone(), viewer_auth)
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit-events")
                .header("authorization", viewer_authorization)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

async fn assert_identity_tenant_revocation_and_audit_are_fail_closed(pool: &PgPool) {
    let operator_id = OperatorId::from_uuid(Uuid::new_v4());
    let other_operator_id = Uuid::new_v4();
    let subject = format!("identity-{}", Uuid::new_v4());
    let tenant = format!("tenant-{}", Uuid::new_v4());
    let other_tenant = format!("tenant-{}", Uuid::new_v4());
    let store = AuthStore::new(pool.clone());
    store
        .bootstrap_development(&DevelopmentIdentity {
            operator_id,
            operator_code: format!("A{}", &operator_id.as_uuid().simple().to_string()[..6]),
            operator_name: "Tenant A".into(),
            external_tenant_id: tenant.clone(),
            subject: subject.clone(),
            display_name: "Tenant Administrator".into(),
            role: AuthRole::Administrator,
        })
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO operators (id, code, display_name, identity_provider, external_tenant_id) VALUES ($1,$2,'Tenant B','development',$3)",
    )
    .bind(other_operator_id)
    .bind(format!("B{}", &other_operator_id.simple().to_string()[..6]))
    .bind(&other_tenant)
    .execute(pool)
    .await
    .unwrap();

    let now = Utc::now();
    let claims = AssertionClaims {
        iss: "test-web".into(),
        aud: "test-api".into(),
        sub: subject.clone(),
        provider: "development".into(),
        tenant: tenant.clone(),
        sid: Uuid::new_v4().to_string(),
        jti: Uuid::new_v4().to_string(),
        iat: now.timestamp() as u64,
        nbf: now.timestamp() as u64,
        exp: (now + Duration::minutes(5)).timestamp() as u64,
    };
    let context = store.resolve(&claims).await.unwrap();
    assert_eq!(context.operator_id, operator_id);

    let mut cross_tenant = claims.clone();
    cross_tenant.tenant = other_tenant;
    assert!(store.resolve(&cross_tenant).await.is_err());

    store
        .revoke_session(
            &context,
            &SessionRevocation {
                provider: "development".into(),
                session_id: claims.sid.clone(),
                identity_id: context.identity_id,
                reason: "contract revocation".into(),
                expires_at: now + Duration::minutes(5),
                requested_at: now,
            },
        )
        .await
        .unwrap();
    assert!(store.resolve(&claims).await.is_err());
    let audit = sqlx::query_as::<_, (Uuid, Uuid, String)>(
        "SELECT operator_id, actor_identity_id, action FROM authorization_audit_events WHERE target_id = $1",
    )
    .bind(&claims.sid)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        audit,
        (
            operator_id.as_uuid(),
            context.identity_id,
            "session.revoked".into()
        )
    );
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
    AuthStore::new(pool.clone())
        .bootstrap_development(&DevelopmentIdentity {
            operator_id: scenario.operator_id,
            operator_code: "ALERT".into(),
            operator_name: "Alert Test".into(),
            external_tenant_id: format!("alert-test-{}", scenario.operator_id.as_uuid()),
            subject: "dispatcher:queue-test".into(),
            display_name: "Queue Test Dispatcher".into(),
            role: AuthRole::Dispatcher,
        })
        .await
        .unwrap();
    let assignee = store.list_assignees(scenario.operator_id).await.unwrap()[0].clone();
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
    let ranked = store
        .list_queue(
            scenario.operator_id,
            &AlertQueueFilter {
                limit: 200,
                ..Default::default()
            },
        )
        .await
        .unwrap();
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
        expected_workflow_version: 1,
        comment: None,
        assigned_identity_id: None,
        dismissal_reason: None,
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

    let other_operator_id = OperatorId::from_uuid(Uuid::new_v4());
    AuthStore::new(pool.clone())
        .bootstrap_development(&DevelopmentIdentity {
            operator_id: other_operator_id,
            operator_code: format!(
                "Q{}",
                &other_operator_id.as_uuid().simple().to_string()[..6]
            ),
            operator_name: "Other Queue Tenant".into(),
            external_tenant_id: format!("other-queue-{}", other_operator_id.as_uuid()),
            subject: "dispatcher:other-tenant".into(),
            display_name: "Other Tenant Dispatcher".into(),
            role: AuthRole::Dispatcher,
        })
        .await
        .unwrap();
    let other_assignee = store.list_assignees(other_operator_id).await.unwrap()[0].clone();

    let direct_alert_error = sqlx::query(
        r#"
        UPDATE alerts
        SET assigned_identity_id = $3,
            assigned_at = $4,
            assigned_by_actor_id = 'direct-database-test'
        WHERE operator_id = $1 AND id = $2
        "#,
    )
    .bind(scenario.operator_id.as_uuid())
    .bind(revised_id)
    .bind(other_assignee.identity_id)
    .bind(evaluated_at)
    .execute(pool)
    .await
    .unwrap_err();
    assert_foreign_key_violation(direct_alert_error, "alerts_assigned_membership_fk");

    let direct_action_error = sqlx::query(
        r#"
        INSERT INTO alert_actions (
            id, operator_id, alert_id, schema_version, action, actor_id,
            occurred_at, idempotency_key, assigned_identity_id
        ) VALUES ($1, $2, $3, 1, 'assign', 'direct-database-test', $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(scenario.operator_id.as_uuid())
    .bind(revised_id)
    .bind(evaluated_at)
    .bind(format!("direct-cross-tenant-{}", Uuid::new_v4()))
    .bind(other_assignee.identity_id)
    .execute(pool)
    .await
    .unwrap_err();
    assert_foreign_key_violation(direct_action_error, "alert_actions_assigned_membership_fk");

    let cross_tenant_assign = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Assign,
        actor_id: "dispatcher:test".into(),
        idempotency_key: "cross-tenant-assign".into(),
        expected_workflow_version: 2,
        comment: None,
        assigned_identity_id: Some(other_assignee.identity_id),
        dismissal_reason: None,
    };
    assert!(matches!(
        store
            .apply_action(revised_id, &cross_tenant_assign, evaluated_at)
            .await,
        Err(AlertStoreError::InvalidAssignee)
    ));

    let (auth, authorization) = authenticated_service(pool, scenario.operator_id).await;
    let api_response = build_router(pool.clone(), auth)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/alerts/{revised_id}/actions"))
                .header("authorization", authorization)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "action": "assign",
                        "idempotency_key": "api-cross-tenant-assign",
                        "expected_workflow_version": 2,
                        "comment": null,
                        "assigned_identity_id": other_assignee.identity_id,
                        "dismissal_reason": null
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(api_response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let api_body: Value =
        serde_json::from_slice(&api_response.into_body().collect().await.unwrap().to_bytes())
            .unwrap();
    assert_eq!(api_body["error"]["code"], "invalid_assignee");

    let assign = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Assign,
        actor_id: "dispatcher:test".into(),
        idempotency_key: "assign-revised-alert".into(),
        expected_workflow_version: 2,
        comment: None,
        assigned_identity_id: Some(assignee.identity_id),
        dismissal_reason: None,
    };
    let assigned = store
        .apply_action(revised_id, &assign, evaluated_at)
        .await
        .unwrap();
    assert_eq!(
        assigned.alert.assigned_identity_id,
        Some(assignee.identity_id)
    );
    assert_eq!(assigned.alert.workflow_version, 3);

    let stale_comment = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Comment,
        actor_id: "dispatcher:stale".into(),
        idempotency_key: "stale-comment-revised-alert".into(),
        expected_workflow_version: 2,
        comment: Some("This copy is stale".into()),
        assigned_identity_id: None,
        dismissal_reason: None,
    };
    assert!(matches!(
        store
            .apply_action(revised_id, &stale_comment, evaluated_at)
            .await,
        Err(AlertStoreError::ConcurrentModification)
    ));

    let assigned_filter = store
        .list_queue(
            scenario.operator_id,
            &AlertQueueFilter {
                severity: Some("warning".into()),
                flight: flight.callsign.clone(),
                event_from: Some(evaluated_at - Duration::seconds(1)),
                event_to: Some(evaluated_at + Duration::seconds(1)),
                assignment: Some(AssignmentFilter::Identity(assignee.identity_id)),
                limit: 200,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(assigned_filter.len(), 1);
    assert_eq!(assigned_filter[0].id, revised_id);

    let comment = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Comment,
        actor_id: "dispatcher:test".into(),
        idempotency_key: "comment-revised-alert".into(),
        expected_workflow_version: 3,
        comment: Some("Coordinating with the flight crew".into()),
        assigned_identity_id: None,
        dismissal_reason: None,
    };
    let commented = store
        .apply_action(revised_id, &comment, evaluated_at)
        .await
        .unwrap();
    assert_eq!(commented.alert.lifecycle, "acknowledged");
    assert_eq!(commented.actions.len(), 3);

    let missing_reason = AlertActionRequest {
        operator_id: scenario.operator_id,
        action: AlertActionKind::Dismiss,
        actor_id: "dispatcher:test".into(),
        idempotency_key: "invalid-dismiss-revised-alert".into(),
        expected_workflow_version: 4,
        comment: None,
        assigned_identity_id: None,
        dismissal_reason: None,
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
        expected_workflow_version: 4,
        comment: Some("Duplicate dispatch information".into()),
        assigned_identity_id: None,
        dismissal_reason: Some(DismissalReason::DuplicateAlert),
    };
    let dismissed = store
        .apply_action(revised_id, &dismiss, evaluated_at)
        .await
        .unwrap();
    assert_eq!(dismissed.alert.lifecycle, "dismissed");
    assert_eq!(dismissed.actions.len(), 4);

    let current = store
        .list_queue(
            scenario.operator_id,
            &AlertQueueFilter {
                limit: 200,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].id, low_priority_id);
    let history = store
        .list_queue(
            scenario.operator_id,
            &AlertQueueFilter {
                include_terminal: true,
                limit: 200,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(history.iter().any(|alert| alert.id == revised_id));
    assert!(!history.iter().any(|alert| alert.id == created_id));

    let dismissed_only = store
        .list_queue(
            scenario.operator_id,
            &AlertQueueFilter {
                lifecycle: Some("dismissed".into()),
                limit: 200,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(dismissed_only.len(), 1);
    assert_eq!(dismissed_only[0].id, revised_id);

    for ordinal in 0..120 {
        let id = Uuid::new_v4();
        let series_key = format!("representative-volume-{ordinal}-{id}");
        sqlx::query(
            r#"
            INSERT INTO alerts (
                id,operator_id,schema_version,event_time,received_at,processed_at,
                alert_type,severity,lifecycle,rule_id,rule_version,dedupe_key,
                series_key,alert_revision,attention_score,score_version,evidence
            ) VALUES ($1,$2,1,$3,$3,$3,'volume_test','information','open',
                      'volume_rule',1,$4,$4,1,5,1,'{}')
            "#,
        )
        .bind(id)
        .bind(scenario.operator_id.as_uuid())
        .bind(evaluated_at + Duration::seconds(i64::from(ordinal)))
        .bind(series_key)
        .execute(pool)
        .await
        .unwrap();
    }
    let representative_page = store
        .list_queue(
            scenario.operator_id,
            &AlertQueueFilter {
                severity: Some("information".into()),
                assignment: Some(AssignmentFilter::Unassigned),
                limit: 100,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(representative_page.len(), 100);
    assert!(
        representative_page
            .iter()
            .all(|alert| alert.assigned_identity_id.is_none())
    );
}

fn assert_foreign_key_violation(error: sqlx::Error, constraint: &str) {
    let sqlx::Error::Database(database_error) = error else {
        panic!("expected database constraint error, got {error}")
    };
    assert_eq!(database_error.code().as_deref(), Some("23503"));
    assert_eq!(database_error.constraint(), Some(constraint));
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
    let (auth, authorization) = authenticated_service(pool, operator_id).await;
    let app = build_router(pool.clone(), auth);
    let system_health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/system/health")
                .header("authorization", &authorization)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(system_health.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &system_health
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes(),
    )
    .unwrap();
    assert_eq!(body["service"], "flight-tracker-api");
    assert!(body["workers"].is_array());

    let system_readiness = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/system/readiness")
                .header("authorization", &authorization)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(system_readiness.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &system_readiness
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes(),
    )
    .unwrap();
    assert_eq!(body["checks"]["database"], "ok");
    assert_eq!(body["checks"]["postgis"], "ok");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/source-health")
                .header("authorization", &authorization)
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

    let (auth, authorization) = authenticated_service(pool, operator_id).await;
    let app = build_router(pool.clone(), auth);
    let observations = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/airport-observations")
                .header("authorization", &authorization)
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
                .header("authorization", &authorization)
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
                .header("authorization", &authorization)
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

    let other_operator = OperatorId::from_uuid(Uuid::new_v4());
    let (other_auth, other_authorization) = authenticated_service(pool, other_operator).await;
    let other_app = build_router(pool.clone(), other_auth);
    for path in [
        "/api/source-health",
        "/api/airport-observations",
        "/api/hazards",
        "/api/alerts",
    ] {
        let response = other_app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("authorization", &other_authorization)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(
            body["data"].as_array().unwrap().len(),
            0,
            "{path} leaked tenant data"
        );
    }
    let source = other_app
        .oneshot(
            Request::builder()
                .uri(format!("/api/source-records/{metar_source_id}"))
                .header("authorization", &other_authorization)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source.status(), StatusCode::NOT_FOUND);

    cleanup_noaa_test_records(pool, operator_uuid).await;
    cleanup_noaa_test_records(pool, other_operator.as_uuid()).await;
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
    sqlx::query("DELETE FROM authorization_audit_events WHERE operator_id = $1")
        .bind(operator_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM auth_session_revocations WHERE operator_id = $1")
        .bind(operator_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM operator_memberships WHERE operator_id = $1")
        .bind(operator_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM auth_identities identity WHERE NOT EXISTS (SELECT 1 FROM operator_memberships membership WHERE membership.identity_id = identity.id)",
    )
    .execute(pool)
    .await
    .unwrap();
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
