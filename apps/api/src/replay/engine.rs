use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{domain::OperatorId, ingestion::NormalizedEventBatch};

use super::{ReplayScenario, ScenarioError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplaySpeed {
    #[serde(rename = "0.25x")]
    Quarter,
    #[serde(rename = "0.5x")]
    Half,
    #[serde(rename = "1x")]
    #[default]
    Normal,
    #[serde(rename = "2x")]
    Double,
    #[serde(rename = "4x")]
    Quadruple,
    #[serde(rename = "8x")]
    Octuple,
}

impl ReplaySpeed {
    const fn ratio(self) -> (u128, u128) {
        match self {
            Self::Quarter => (1, 4),
            Self::Half => (1, 2),
            Self::Normal => (1, 1),
            Self::Double => (2, 1),
            Self::Quadruple => (4, 1),
            Self::Octuple => (8, 1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayPhase {
    Paused,
    Running,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayStatus {
    pub scenario_id: String,
    pub phase: ReplayPhase,
    pub speed: ReplaySpeed,
    pub cursor: usize,
    pub total_events: usize,
    pub emitted_events: usize,
    pub virtual_elapsed_ms: u64,
    pub virtual_time: DateTime<Utc>,
    pub feed_outage: bool,
}

pub struct ReplayEngine {
    scenario: ReplayScenario,
    phase: ReplayPhase,
    speed: ReplaySpeed,
    cursor: usize,
    elapsed_ms: u64,
    fractional_remainder: u128,
    feed_outage: bool,
}

impl ReplayEngine {
    pub fn new(scenario: ReplayScenario) -> Self {
        Self {
            scenario,
            phase: ReplayPhase::Paused,
            speed: ReplaySpeed::Normal,
            cursor: 0,
            elapsed_ms: 0,
            fractional_remainder: 0,
            feed_outage: false,
        }
    }

    pub fn status(&self) -> ReplayStatus {
        let elapsed = chrono::Duration::try_milliseconds(self.elapsed_ms as i64)
            .unwrap_or(chrono::Duration::MAX);
        ReplayStatus {
            scenario_id: self.scenario.id.clone(),
            phase: self.phase,
            speed: self.speed,
            cursor: self.cursor,
            total_events: self.scenario.events.len(),
            emitted_events: self.cursor,
            virtual_elapsed_ms: self.elapsed_ms,
            virtual_time: self.scenario.start_time + elapsed,
            feed_outage: self.feed_outage,
        }
    }

    pub const fn operator_id(&self) -> OperatorId {
        self.scenario.operator_id
    }

    pub fn pause(&mut self) -> ReplayStatus {
        if self.phase == ReplayPhase::Running {
            self.phase = ReplayPhase::Paused;
        }
        self.status()
    }

    pub fn resume(&mut self) -> ReplayStatus {
        if self.cursor < self.scenario.events.len() {
            self.phase = ReplayPhase::Running;
        }
        self.status()
    }

    pub fn reset(&mut self) -> ReplayStatus {
        self.phase = ReplayPhase::Paused;
        self.cursor = 0;
        self.elapsed_ms = 0;
        self.fractional_remainder = 0;
        self.feed_outage = false;
        self.status()
    }

    pub fn set_speed(&mut self, speed: ReplaySpeed) -> ReplayStatus {
        self.speed = speed;
        self.fractional_remainder = 0;
        self.status()
    }

    pub fn set_feed_outage(&mut self, active: bool) -> ReplayStatus {
        self.feed_outage = active;
        self.status()
    }

    pub fn advance(
        &mut self,
        wall_delta: Duration,
    ) -> Result<Vec<NormalizedEventBatch>, ScenarioError> {
        if self.phase != ReplayPhase::Running || self.feed_outage {
            return Ok(Vec::new());
        }

        let (numerator, denominator) = self.speed.ratio();
        let scaled = wall_delta.as_millis() * numerator + self.fractional_remainder;
        let additional_ms = scaled / denominator;
        self.fractional_remainder = scaled % denominator;
        self.elapsed_ms = self
            .elapsed_ms
            .saturating_add(u64::try_from(additional_ms).unwrap_or(u64::MAX));

        let mut batches = Vec::new();
        while let Some(event) = self.scenario.events.get(self.cursor) {
            if event.offset_ms > self.elapsed_ms {
                break;
            }
            batches.push(self.scenario.batch_for(event)?);
            self.cursor += 1;
        }
        if self.cursor == self.scenario.events.len() {
            self.phase = ReplayPhase::Completed;
        }
        Ok(batches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ReplayScenario {
        ReplayScenario::from_json(include_str!(
            "../../../../fixtures/replay/m1-operations-v1.json"
        ))
        .unwrap()
    }

    #[test]
    fn reset_replays_identical_normalized_events() {
        let mut engine = ReplayEngine::new(fixture());
        engine.resume();
        let first = engine.advance(Duration::from_secs(600)).unwrap();
        engine.reset();
        engine.resume();
        let second = engine.advance(Duration::from_secs(600)).unwrap();

        assert_eq!(first, second);
        assert_eq!(engine.status().phase, ReplayPhase::Completed);
    }

    #[test]
    fn pause_and_speed_control_virtual_time() {
        let mut engine = ReplayEngine::new(fixture());
        assert!(engine.advance(Duration::from_secs(1)).unwrap().is_empty());
        assert_eq!(engine.status().virtual_elapsed_ms, 0);

        engine.set_speed(ReplaySpeed::Double);
        engine.resume();
        engine.advance(Duration::from_millis(500)).unwrap();
        assert_eq!(engine.status().virtual_elapsed_ms, 1_000);

        engine.pause();
        engine.advance(Duration::from_secs(1)).unwrap();
        assert_eq!(engine.status().virtual_elapsed_ms, 1_000);
    }

    #[test]
    fn simulated_feed_outage_suspends_events_until_restored() {
        let mut engine = ReplayEngine::new(fixture());
        engine.resume();
        let outage = engine.set_feed_outage(true);
        assert!(outage.feed_outage);
        assert!(engine.advance(Duration::from_secs(60)).unwrap().is_empty());
        assert_eq!(engine.status().virtual_elapsed_ms, 0);

        engine.set_feed_outage(false);
        assert!(!engine.advance(Duration::from_secs(60)).unwrap().is_empty());
        assert_eq!(engine.status().virtual_elapsed_ms, 60_000);
    }
}
