use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use serde::Serialize;

const DEFAULT_MAX_HEARTBEAT_AGE: Duration = Duration::from_secs(3);

#[derive(Clone, Default)]
pub struct CriticalWorkerRegistry {
    workers: Arc<RwLock<BTreeMap<&'static str, WorkerRecord>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerState {
    Starting,
    Running,
    Stale,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkerSnapshot {
    pub name: &'static str,
    pub state: WorkerState,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub detail: Option<&'static str>,
}

impl WorkerSnapshot {
    pub fn is_ready(&self) -> bool {
        self.state == WorkerState::Running
    }
}

#[derive(Debug, Clone)]
struct WorkerRecord {
    state: WorkerState,
    last_heartbeat_at: Option<DateTime<Utc>>,
    last_heartbeat: Option<Instant>,
    detail: Option<&'static str>,
}

impl CriticalWorkerRegistry {
    pub fn register(&self, name: &'static str) -> WorkerProbe {
        self.workers
            .write()
            .expect("worker registry poisoned")
            .insert(
                name,
                WorkerRecord {
                    state: WorkerState::Starting,
                    last_heartbeat_at: None,
                    last_heartbeat: None,
                    detail: None,
                },
            );
        WorkerProbe {
            name,
            registry: self.clone(),
            terminal: false,
        }
    }

    pub fn snapshot(&self) -> Vec<WorkerSnapshot> {
        self.snapshot_with_max_age(DEFAULT_MAX_HEARTBEAT_AGE)
    }

    pub fn is_ready(&self) -> bool {
        self.snapshot().iter().all(WorkerSnapshot::is_ready)
    }

    fn snapshot_with_max_age(&self, max_age: Duration) -> Vec<WorkerSnapshot> {
        self.workers
            .read()
            .expect("worker registry poisoned")
            .iter()
            .map(|(name, record)| {
                let state = if record.state == WorkerState::Running
                    && record
                        .last_heartbeat
                        .is_some_and(|heartbeat| heartbeat.elapsed() > max_age)
                {
                    WorkerState::Stale
                } else {
                    record.state
                };
                WorkerSnapshot {
                    name,
                    state,
                    last_heartbeat_at: record.last_heartbeat_at,
                    detail: record.detail,
                }
            })
            .collect()
    }

    fn update(
        &self,
        name: &'static str,
        state: WorkerState,
        heartbeat: bool,
        detail: Option<&'static str>,
    ) {
        if let Some(record) = self
            .workers
            .write()
            .expect("worker registry poisoned")
            .get_mut(name)
        {
            record.state = state;
            record.detail = detail;
            if heartbeat {
                record.last_heartbeat = Some(Instant::now());
                record.last_heartbeat_at = Some(Utc::now());
            }
        }
    }
}

pub struct WorkerProbe {
    name: &'static str,
    registry: CriticalWorkerRegistry,
    terminal: bool,
}

impl WorkerProbe {
    pub fn heartbeat(&self) {
        self.registry
            .update(self.name, WorkerState::Running, true, None);
    }

    pub fn fail(&mut self, detail: &'static str) {
        self.terminal = true;
        self.registry
            .update(self.name, WorkerState::Failed, false, Some(detail));
    }
}

impl Drop for WorkerProbe {
    fn drop(&mut self) {
        if !self.terminal {
            self.registry.update(
                self.name,
                WorkerState::Stopped,
                false,
                Some("worker task stopped"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_workers_must_heartbeat_before_readiness() {
        let registry = CriticalWorkerRegistry::default();
        let probe = registry.register("projection");
        assert!(!registry.is_ready());
        assert_eq!(registry.snapshot()[0].state, WorkerState::Starting);

        probe.heartbeat();
        assert!(registry.is_ready());
        assert_eq!(registry.snapshot()[0].state, WorkerState::Running);
    }

    #[test]
    fn stale_failed_and_stopped_workers_fail_closed() {
        let registry = CriticalWorkerRegistry::default();
        let mut failed = registry.register("failed");
        failed.fail("test failure");
        let stopped = registry.register("stopped");
        stopped.heartbeat();
        drop(stopped);
        let stale = registry.register("stale");
        stale.heartbeat();

        let snapshot = registry.snapshot_with_max_age(Duration::ZERO);
        assert_eq!(snapshot[0].state, WorkerState::Failed);
        assert_eq!(snapshot[1].state, WorkerState::Stale);
        assert_eq!(snapshot[2].state, WorkerState::Stopped);
        assert!(!registry.is_ready());
    }
}
