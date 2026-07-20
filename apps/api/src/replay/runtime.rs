use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, task::JoinHandle};

use crate::ingestion::{IngestionHub, NormalizedEventBatch};

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

    async fn advance(&self, elapsed: Duration) -> Result<(), ScenarioError> {
        let batches = self.engine.lock().await.advance(elapsed)?;
        for batch in batches {
            self.ingestion.publish(batch);
        }
        Ok(())
    }
}

pub fn spawn_replay_runtime(handle: ReplayHandle, tick: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tick);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = handle.advance(tick).await {
                tracing::error!(error = %error, "replay scenario failed");
                handle.pause().await;
            }
        }
    })
}
