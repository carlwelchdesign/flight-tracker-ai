mod config;

use std::time::Duration;

use config::Config;
use flight_tracker_api::{
    alerting::spawn_alert_worker,
    auth::{AuthService, AuthStore, InternalAssertionVerifier},
    build_router_with_runtime_and_ingestion,
    health::CriticalWorkerRegistry,
    ingestion::{IngestionHub, IngestionSubscription},
    replay::{ReplayHandle, ReplayScenario, spawn_replay_runtime},
    retention::{RetentionStore, spawn_retention_scheduler},
    weather::noaa::{
        NoaaClient, NoaaClientConfig, NoaaRuntimeConfig, NoaaStore, RetryPolicy, spawn_noaa_runtime,
    },
};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "flight_tracker_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let config = Config::from_env()?;
    let database = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&database).await?;

    let auth_store = AuthStore::new(database.clone());
    if let Some(identity) = config.auth.development_identity.as_ref() {
        auth_store.bootstrap_development(identity).await?;
    }
    let auth = AuthService::new(
        InternalAssertionVerifier::new(config.auth.assertion.clone())?,
        auth_store,
    );

    let workers = CriticalWorkerRegistry::default();
    let mut ingestion_subscriptions = Vec::<IngestionSubscription>::new();
    let replay = if let Some(replay_config) = config.replay {
        let scenario = ReplayScenario::load(&replay_config.scenario_path)?;
        let scenario_id = scenario.id.clone();
        let handle = ReplayHandle::new(scenario, 256);
        spawn_replay_runtime(
            handle.clone(),
            Duration::from_millis(100),
            workers.register("replay_runtime"),
        );
        tracing::info!(
            correlation_id = %scenario_id,
            scenario = %scenario_id,
            "development replay controls enabled"
        );
        Some(handle)
    } else {
        None
    };

    if let Some(noaa_config) = config.noaa {
        let operator_exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM operators WHERE id = $1)")
                .bind(noaa_config.operator_id.as_uuid())
                .fetch_one(&database)
                .await?;
        if !operator_exists {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "NOAA_OPERATOR_ID must reference an existing operator",
            )
            .into());
        }
        let ingestion = IngestionHub::new(256);
        ingestion_subscriptions.push(ingestion.subscription("noaa_projection"));
        spawn_alert_worker(
            database.clone(),
            ingestion.subscribe(),
            workers.register("noaa_alert_projection"),
        );
        let client = NoaaClient::new(NoaaClientConfig {
            base_url: noaa_config.base_url,
            user_agent: noaa_config.user_agent,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
            retry: RetryPolicy::default(),
        })?;
        spawn_noaa_runtime(
            client,
            NoaaStore::new(database.clone()),
            ingestion,
            NoaaRuntimeConfig {
                operator_id: noaa_config.operator_id,
                stations: noaa_config.stations,
                poll_interval: noaa_config.poll_interval,
                metar_stale_after: noaa_config.metar_stale_after,
                air_sigmet_stale_after: noaa_config.air_sigmet_stale_after,
            },
            workers.register("noaa_weather_ingestion"),
        )?;
        tracing::info!("NOAA aviation weather ingestion enabled");
    }

    spawn_retention_scheduler(
        RetentionStore::new(database.clone()),
        Duration::from_secs(30),
        workers.register("retention_scheduler"),
    );

    let listener = TcpListener::bind(config.bind_address).await?;
    tracing::info!(address = %config.bind_address, "API listening");
    axum::serve(
        listener,
        build_router_with_runtime_and_ingestion(
            database,
            replay,
            workers,
            ingestion_subscriptions,
            auth,
        ),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install termination signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
