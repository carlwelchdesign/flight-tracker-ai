mod config;

use std::time::Duration;

use config::Config;
use flight_tracker_api::{
    build_router_with_replay,
    replay::{ReplayHandle, ReplayScenario, spawn_replay_runtime},
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
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let database = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&database).await?;

    let replay = if let Some(replay_config) = config.replay {
        let scenario = ReplayScenario::load(&replay_config.scenario_path)?;
        let scenario_id = scenario.id.clone();
        let handle = ReplayHandle::new(scenario, 256);
        spawn_replay_runtime(handle.clone(), Duration::from_millis(100));
        tracing::info!(scenario = %scenario_id, "development replay controls enabled");
        Some(handle)
    } else {
        None
    };

    let listener = TcpListener::bind(config.bind_address).await?;
    tracing::info!(address = %config.bind_address, "API listening");
    axum::serve(listener, build_router_with_replay(database, replay))
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
