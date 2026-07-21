use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::domain::{CanonicalEvent, ProviderEnvelope};

/// The provider-independent handoff consumed by projections, persistence, and
/// live streams. Replay and future live adapters publish the same contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedEventBatch {
    pub envelope: ProviderEnvelope,
    pub events: Vec<CanonicalEvent>,
}

#[derive(Debug, Clone)]
pub struct IngestionHub {
    sender: broadcast::Sender<Arc<NormalizedEventBatch>>,
}

pub struct IngestionSubscription {
    pub worker_name: &'static str,
    pub receiver: broadcast::Receiver<Arc<NormalizedEventBatch>>,
}

impl IngestionHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<NormalizedEventBatch>> {
        self.sender.subscribe()
    }

    pub fn subscription(&self, worker_name: &'static str) -> IngestionSubscription {
        IngestionSubscription {
            worker_name,
            receiver: self.subscribe(),
        }
    }

    pub fn publish(&self, batch: NormalizedEventBatch) -> usize {
        self.sender.send(Arc::new(batch)).unwrap_or_default()
    }
}
