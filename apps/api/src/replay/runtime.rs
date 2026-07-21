use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    health::WorkerProbe,
    ingestion::{IngestionHub, NormalizedEventBatch},
};

use super::{ReplayEngine, ReplayScenario, ReplaySpeed, ReplayStatus, ScenarioError};

#[derive(Clone)]
pub struct ReplayHandle {
    engine: Arc<Mutex<ReplayEngine>>,
    ingestion: IngestionHub,
}

impl ReplayHandle {
    pub fn new(scenario: ReplayScenario, channel_capacity: usize) -> Self {
        Self {
            engine: Arc::new(Mutex::new(ReplayEngine::new(scenario))),
            ingestion: IngestionHub::new(channel_capacity),
        }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Arc<NormalizedEventBatch>> {
        self.ingestion.subscribe()
    }

    pub async fn status(&self) -> ReplayStatus {
        self.engine.lock().await.status()
    }

    pub async fn pause(&self) -> ReplayStatus {
        self.engine.lock().await.pause()
    }

    pub async fn resume(&self) -> ReplayStatus {
        self.engine.lock().await.resume()
    }

    pub async fn reset(&self) -> ReplayStatus {
        self.engine.lock().await.reset()
    }

    pub async fn set_speed(&self, speed: ReplaySpeed) -> ReplayStatus {
        self.engine.lock().await.set_speed(speed)
    }

    pub async fn set_feed_outage(&self, active: bool) -> ReplayStatus {
        self.engine.lock().await.set_feed_outage(active)
    }

    async fn advance(&self, elapsed: Duration) -> Result<(), ScenarioError> {
        let batches = self.engine.lock().await.advance(elapsed)?;
        for batch in batches {
            self.ingestion.publish(batch);
        }
        Ok(())
    }
}

pub fn spawn_replay_runtime(
    handle: ReplayHandle,
    tick: Duration,
    mut probe: WorkerProbe,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let scenario_id = handle.status().await.scenario_id;
        let mut interval = tokio::time::interval(tick);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        probe.heartbeat();
        loop {
            interval.tick().await;
            if let Err(error) = handle.advance(tick).await {
                tracing::error!(
                    correlation_id = %scenario_id,
                    worker = "replay_runtime",
                    error = %error,
                    "replay scenario failed"
                );
                handle.pause().await;
                probe.fail("replay scenario failed");
                break;
            }
            probe.heartbeat();
        }
    })
}
