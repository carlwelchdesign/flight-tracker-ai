use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use flight_tracker_api::{
    alerting::{AlertActionRequest, AlertStore, spawn_alert_worker},
    domain::{AlertActionKind, FlightId, OperatorId, PlannedRouteId, WeatherHazardId},
    health::CriticalWorkerRegistry,
    ingestion::IngestionHub,
    replay::{ReplayScenario, ScenarioPayload},
};
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

const PRODUCTION_REPLAY_CHANNEL_CAPACITY: usize = 256;
const BOUNDED_BACKLOG_CYCLES: usize = 15;
const DRILL_TIMEOUT: Duration = Duration::from_secs(20);

/// This drill uses the real PostGIS schema when TEST_DATABASE_URL is present.
/// Unit-only environments skip it rather than substituting an in-memory store.
#[tokio::test]
async fn worker_restart_and_bounded_backlog_preserve_alert_history() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL not set; skipping FT-402 PostGIS resilience drill");
        return;
    };
    let pool = PgPool::connect(&database_url).await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let scenario_a = isolated_scenario("restart-a");
    let first_hub = IngestionHub::new(PRODUCTION_REPLAY_CHANNEL_CAPACITY);
    let workers = CriticalWorkerRegistry::default();
    let first_worker = spawn_alert_worker(
        pool.clone(),
        first_hub.subscribe(),
        workers.register("ft402_alert_projection"),
    );
    publish_scenario(&first_hub, &scenario_a);
    let (alert_a, _) = wait_for_alert(&pool, scenario_a.operator_id).await;

    let store = AlertStore::new(pool.clone());
    let commented = store
        .apply_action(
            alert_a,
            &AlertActionRequest {
                operator_id: scenario_a.operator_id,
                action: AlertActionKind::Comment,
                actor_id: "ft402:controlled-drill".into(),
                idempotency_key: format!("ft402-restart-comment-{alert_a}"),
                expected_workflow_version: 1,
                comment: Some("Controlled restart marker".into()),
                assigned_identity_id: None,
                dismissal_reason: None,
            },
            scenario_a.start_time,
        )
        .await
        .unwrap();
    assert_eq!(commented.alert.workflow_version, 2);
    assert_eq!(commented.actions.len(), 1);

    first_worker.abort();
    let _ = first_worker.await;

    let scenario_b = isolated_scenario("restart-b");
    let restart_hub = IngestionHub::new(PRODUCTION_REPLAY_CHANNEL_CAPACITY);
    let restart_receiver = restart_hub.subscribe();
    for _ in 0..BOUNDED_BACKLOG_CYCLES {
        publish_scenario(&restart_hub, &scenario_a);
    }
    publish_scenario(&restart_hub, &scenario_b);
    let bounded_batches =
        BOUNDED_BACKLOG_CYCLES * scenario_a.events.len() + scenario_b.events.len();
    assert!(bounded_batches < PRODUCTION_REPLAY_CHANNEL_CAPACITY);

    let bounded_started = Instant::now();
    let restarted_worker = spawn_alert_worker(
        pool.clone(),
        restart_receiver,
        workers.register("ft402_alert_projection"),
    );
    let (_, bounded_elapsed) = wait_for_alert(&pool, scenario_b.operator_id).await;
    assert!(bounded_started.elapsed() < DRILL_TIMEOUT);
    assert_history_is_unchanged(&store, scenario_a.operator_id, alert_a).await;
    assert_eq!(alert_count(&pool, scenario_a.operator_id).await, 1);
    assert_eq!(
        envelope_count(&pool, scenario_a.operator_id).await,
        i64::try_from(scenario_a.events.len()).unwrap()
    );
    println!(
        "FT402_BACKLOG bounded_batches={bounded_batches} drain_ms={} result=passed",
        bounded_elapsed.as_millis()
    );
    restarted_worker.abort();
    let _ = restarted_worker.await;

    let scenario_c = isolated_scenario("overflow-c");
    let overflow_hub = IngestionHub::new(16);
    let mut measurement_receiver = overflow_hub.subscribe();
    let worker_receiver = overflow_hub.subscribe();
    for _ in 0..20 {
        publish_scenario(&overflow_hub, &scenario_a);
    }
    publish_scenario(&overflow_hub, &scenario_c);
    let published_batches = 20 * scenario_a.events.len() + scenario_c.events.len();
    let skipped = match measurement_receiver.recv().await {
        Err(broadcast::error::RecvError::Lagged(skipped)) => skipped,
        other => panic!("overflow probe must report a lagged receiver, got {other:?}"),
    };
    assert!(skipped > 0);

    let overflow_started = Instant::now();
    let overflow_worker = spawn_alert_worker(
        pool.clone(),
        worker_receiver,
        workers.register("ft402_alert_projection"),
    );
    let (_, overflow_elapsed) = wait_for_alert(&pool, scenario_c.operator_id).await;
    assert!(overflow_started.elapsed() < DRILL_TIMEOUT);
    assert_history_is_unchanged(&store, scenario_a.operator_id, alert_a).await;
    println!(
        "FT402_BACKLOG published_batches={published_batches} skipped_batches={skipped} recovery_ms={} recovery=complete_replay_window",
        overflow_elapsed.as_millis()
    );
    overflow_worker.abort();
    let _ = overflow_worker.await;
}

fn isolated_scenario(label: &str) -> ReplayScenario {
    let mut scenario = ReplayScenario::from_json(include_str!(
        "../../../fixtures/replay/m1-operations-v1.json"
    ))
    .unwrap();
    scenario.id = format!("ft402-{label}");
    scenario.namespace_id = Uuid::new_v4();
    scenario.operator_id = OperatorId::new();

    let flight_ids = scenario
        .flights
        .iter_mut()
        .map(|flight| {
            let previous = flight.id;
            let replacement = FlightId::new();
            flight.id = replacement;
            (previous, replacement)
        })
        .collect::<HashMap<_, _>>();
    for event in &mut scenario.events {
        event.provider_record_id = format!("{label}-{}", event.provider_record_id);
        match &mut event.payload {
            ScenarioPayload::FlightSnapshot {
                flight_id: value, ..
            }
            | ScenarioPayload::Position {
                flight_id: value, ..
            } => *value = flight_ids[value],
            ScenarioPayload::PlannedRoute {
                route_id,
                flight_id: value,
                ..
            } => {
                *route_id = PlannedRouteId::new();
                *value = flight_ids[value];
            }
            ScenarioPayload::WeatherHazard { hazard_id, .. } => {
                *hazard_id = WeatherHazardId::new();
            }
        }
    }
    scenario.validate().unwrap();
    scenario
}

fn publish_scenario(hub: &IngestionHub, scenario: &ReplayScenario) {
    for event in &scenario.events {
        assert!(hub.publish(scenario.batch_for(event).unwrap()) > 0);
    }
}

async fn wait_for_alert(pool: &PgPool, operator_id: OperatorId) -> (Uuid, Duration) {
    let started = Instant::now();
    let alert_id = tokio::time::timeout(DRILL_TIMEOUT, async {
        loop {
            if let Some(id) = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM alerts WHERE operator_id = $1 ORDER BY alert_revision DESC LIMIT 1",
            )
            .bind(operator_id.as_uuid())
            .fetch_optional(pool)
            .await
            .unwrap()
            {
                break id;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("alert worker did not recover within the bounded drill timeout");
    (alert_id, started.elapsed())
}

async fn assert_history_is_unchanged(store: &AlertStore, operator_id: OperatorId, alert_id: Uuid) {
    let detail = store.detail(operator_id, alert_id).await.unwrap();
    assert_eq!(detail.alert.workflow_version, 2);
    assert_eq!(detail.actions.len(), 1);
    assert_eq!(detail.actions[0].actor_id, "ft402:controlled-drill");
    assert_eq!(
        detail.actions[0].comment.as_deref(),
        Some("Controlled restart marker")
    );
}

async fn alert_count(pool: &PgPool, operator_id: OperatorId) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM alerts WHERE operator_id = $1")
        .bind(operator_id.as_uuid())
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn envelope_count(pool: &PgPool, operator_id: OperatorId) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM provider_envelopes WHERE operator_id = $1")
        .bind(operator_id.as_uuid())
        .fetch_one(pool)
        .await
        .unwrap()
}
