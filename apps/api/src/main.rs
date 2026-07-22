mod config;

use std::time::Duration;

use config::{Config, ReplayConfig};
use flight_tracker_api::{
    PublicPortfolioOperators,
    alerting::spawn_alert_worker,
    auth::{AuthService, AuthStore, InternalAssertionVerifier},
    build_router_with_runtime_and_public_live_positions,
    health::CriticalWorkerRegistry,
    ingestion::{IngestionHub, IngestionSubscription},
    live_positions::{
        AdsbLolClient, AdsbLolClientConfig, AdsbLolRuntimeConfig, LivePositionStatusStore,
        RetryPolicy as AdsbLolRetryPolicy, public_live_region_catalog, spawn_adsb_lol_runtime,
    },
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
    let live_position_statuses = LivePositionStatusStore::default();
    let replay = if let Some(replay_config) = config.replay {
        let (scenario, mode) = match replay_config {
            ReplayConfig::Development { scenario_path } => {
                (ReplayScenario::load(&scenario_path)?, "development")
            }
            ReplayConfig::Portfolio => (
                ReplayScenario::from_json(include_str!(
                    "../../../fixtures/replay/m1-operations-v1.json"
                ))?,
                "portfolio",
            ),
        };
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
            replay_mode = mode,
            "replay controls enabled"
        );
        Some(handle)
    } else {
        None
    };

    let public_weather_operator = config.public_weather_operator;
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

    let public_live_operator = config.adsb_lol.as_ref().map(|value| value.operator_id);
    if let Some(adsb_lol_config) = config.adsb_lol {
        let operator_exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM operators WHERE id = $1)")
                .bind(adsb_lol_config.operator_id.as_uuid())
                .fetch_one(&database)
                .await?;
        if !operator_exists {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "ADSB_LOL_OPERATOR_ID must reference an existing operator",
            )
            .into());
        }
        let ingestion = IngestionHub::new(256);
        ingestion_subscriptions.push(ingestion.subscription("adsb_lol_fleet_projection"));
        let client = AdsbLolClient::new(AdsbLolClientConfig {
            base_url: adsb_lol_config.base_url,
            user_agent: adsb_lol_config.user_agent,
            connect_timeout: Duration::from_secs(3),
            request_timeout: Duration::from_secs(12),
            retry: AdsbLolRetryPolicy::default(),
        })?;
        let regions =
            public_live_region_catalog(adsb_lol_config.operator_id, adsb_lol_config.region);
        for (index, preset) in regions.iter().enumerate() {
            let initial_delay = adsb_lol_config
                .poll_interval
                .mul_f64(index as f64 / regions.len() as f64);
            spawn_adsb_lol_runtime(
                client.clone(),
                ingestion.clone(),
                live_position_statuses.clone(),
                AdsbLolRuntimeConfig {
                    operator_id: preset.operator_id,
                    region: preset.region,
                    initial_delay,
                    poll_interval: adsb_lol_config.poll_interval,
                    stale_after: Duration::from_secs(
                        adsb_lol_config.poll_interval.as_secs().saturating_mul(2),
                    ),
                },
                workers.register(preset.worker_name),
            )?;
        }
        tracing::info!(
            provider = "adsb.lol",
            feed = "point",
            region_count = regions.len(),
            "best-effort regional live aircraft positions enabled"
        );
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
        build_router_with_runtime_and_public_live_positions(
            database,
            replay,
            workers,
            ingestion_subscriptions,
            live_position_statuses,
            PublicPortfolioOperators {
                live_positions: public_live_operator,
                weather: public_weather_operator,
            },
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
