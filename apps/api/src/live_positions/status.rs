use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::OperatorId;

const FEED: &str = "point";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivePositionProvider {
    AdsbLol,
    AirplanesLive,
}

impl LivePositionProvider {
    pub const fn id(self) -> &'static str {
        match self {
            Self::AdsbLol => "adsb.lol",
            Self::AirplanesLive => "airplanes.live",
        }
    }

    pub const fn attribution(self) -> LivePositionAttribution {
        match self {
            Self::AdsbLol => LivePositionAttribution {
                text: "Contains information from ADSB.lol, available under the Open Database License (ODbL).",
                source_name: "ADSB.lol",
                source_url: "https://adsb.lol/",
                terms_label: "Open Database License (ODbL)",
                terms_url: "https://opendatacommons.org/licenses/odbl/1-0/",
            },
            Self::AirplanesLive => LivePositionAttribution {
                text: "Airplanes.live fallback data is displayed under its published non-commercial API terms.",
                source_name: "Airplanes.live",
                source_url: "https://airplanes.live/",
                terms_label: "non-commercial API terms",
                terms_url: "https://airplanes.live/api-guide/",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LivePositionRegion {
    pub latitude_degrees: f64,
    pub longitude_degrees: f64,
    pub radius_nautical_miles: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivePositionState {
    Disabled,
    Connecting,
    Current,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LivePositionAttribution {
    pub text: &'static str,
    pub source_name: &'static str,
    pub source_url: &'static str,
    pub terms_label: &'static str,
    pub terms_url: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LivePositionStatus {
    pub enabled: bool,
    pub provider: Option<&'static str>,
    pub feed: Option<&'static str>,
    pub state: LivePositionState,
    pub best_effort: bool,
    pub observed_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub newest_position_at: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub aircraft_count: usize,
    pub fresh_position_count: usize,
    pub stale_position_count: usize,
    pub rejected_record_count: usize,
    pub missing_callsign_count: usize,
    pub stale_after_seconds: u64,
    pub last_error_code: Option<String>,
    pub region: Option<LivePositionRegion>,
    pub attribution: Option<LivePositionAttribution>,
}

impl LivePositionStatus {
    fn disabled(now: DateTime<Utc>) -> Self {
        Self {
            enabled: false,
            provider: None,
            feed: None,
            state: LivePositionState::Disabled,
            best_effort: true,
            observed_at: now,
            last_attempt_at: None,
            last_success_at: None,
            newest_position_at: None,
            consecutive_failures: 0,
            aircraft_count: 0,
            fresh_position_count: 0,
            stale_position_count: 0,
            rejected_record_count: 0,
            missing_callsign_count: 0,
            stale_after_seconds: 30,
            last_error_code: None,
            region: None,
            attribution: None,
        }
    }

    fn connecting(now: DateTime<Utc>, region: LivePositionRegion, stale_after: Duration) -> Self {
        let provider = LivePositionProvider::AdsbLol;
        Self {
            enabled: true,
            provider: Some(provider.id()),
            feed: Some(FEED),
            state: LivePositionState::Connecting,
            best_effort: true,
            observed_at: now,
            last_attempt_at: None,
            last_success_at: None,
            newest_position_at: None,
            consecutive_failures: 0,
            aircraft_count: 0,
            fresh_position_count: 0,
            stale_position_count: 0,
            rejected_record_count: 0,
            missing_callsign_count: 0,
            stale_after_seconds: stale_after.as_secs(),
            last_error_code: None,
            region: Some(region),
            attribution: Some(provider.attribution()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PositionCoverage {
    pub aircraft_count: usize,
    pub fresh_position_count: usize,
    pub stale_position_count: usize,
    pub rejected_record_count: usize,
    pub missing_callsign_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct LivePositionStatusStore {
    statuses: Arc<RwLock<HashMap<OperatorId, LivePositionStatus>>>,
}

impl LivePositionStatusStore {
    pub(crate) fn register(
        &self,
        operator_id: OperatorId,
        region: LivePositionRegion,
        stale_after: Duration,
        now: DateTime<Utc>,
    ) {
        self.statuses
            .write()
            .expect("live position status lock poisoned")
            .insert(
                operator_id,
                LivePositionStatus::connecting(now, region, stale_after),
            );
    }

    pub(crate) fn record_success(
        &self,
        operator_id: OperatorId,
        provider: LivePositionProvider,
        now: DateTime<Utc>,
        newest_position_at: Option<DateTime<Utc>>,
        coverage: PositionCoverage,
    ) {
        let mut statuses = self
            .statuses
            .write()
            .expect("live position status lock poisoned");
        let Some(status) = statuses.get_mut(&operator_id) else {
            return;
        };
        status.provider = Some(provider.id());
        status.feed = Some(FEED);
        status.attribution = Some(provider.attribution());
        status.state = if coverage.fresh_position_count > 0 {
            LivePositionState::Current
        } else {
            LivePositionState::Degraded
        };
        status.observed_at = now;
        status.last_attempt_at = Some(now);
        status.last_success_at = Some(now);
        status.newest_position_at = newest_position_at;
        status.consecutive_failures = 0;
        status.aircraft_count = coverage.aircraft_count;
        status.fresh_position_count = coverage.fresh_position_count;
        status.stale_position_count = coverage.stale_position_count;
        status.rejected_record_count = coverage.rejected_record_count;
        status.missing_callsign_count = coverage.missing_callsign_count;
        status.last_error_code = None;
    }

    pub(crate) fn record_failure(
        &self,
        operator_id: OperatorId,
        now: DateTime<Utc>,
        error_code: &str,
    ) {
        let mut statuses = self
            .statuses
            .write()
            .expect("live position status lock poisoned");
        let Some(status) = statuses.get_mut(&operator_id) else {
            return;
        };
        status.consecutive_failures = status.consecutive_failures.saturating_add(1);
        status.state = if status.consecutive_failures >= 3 {
            LivePositionState::Unavailable
        } else {
            LivePositionState::Degraded
        };
        status.observed_at = now;
        status.last_attempt_at = Some(now);
        status.last_error_code = Some(error_code.to_owned());
    }

    pub fn snapshot(&self, operator_id: OperatorId, now: DateTime<Utc>) -> LivePositionStatus {
        self.statuses
            .read()
            .expect("live position status lock poisoned")
            .get(&operator_id)
            .cloned()
            .unwrap_or_else(|| LivePositionStatus::disabled(now))
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn now(second: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 21, 17, 0, second).unwrap()
    }

    #[test]
    fn status_is_tenant_scoped_and_transitions_through_failure_and_recovery() {
        let store = LivePositionStatusStore::default();
        let operator = OperatorId::new();
        let other = OperatorId::new();
        let region = LivePositionRegion {
            latitude_degrees: 37.62,
            longitude_degrees: -122.38,
            radius_nautical_miles: 25,
        };
        store.register(operator, region, Duration::from_secs(30), now(0));
        assert_eq!(
            store.snapshot(operator, now(0)).state,
            LivePositionState::Connecting
        );
        assert_eq!(
            store.snapshot(other, now(0)).state,
            LivePositionState::Disabled
        );

        for second in 1..=3 {
            store.record_failure(operator, now(second), "provider_unavailable");
        }
        assert_eq!(
            store.snapshot(operator, now(3)).state,
            LivePositionState::Unavailable
        );

        store.record_success(
            operator,
            LivePositionProvider::AirplanesLive,
            now(4),
            Some(now(3)),
            PositionCoverage {
                aircraft_count: 2,
                fresh_position_count: 1,
                stale_position_count: 1,
                rejected_record_count: 1,
                missing_callsign_count: 1,
            },
        );
        let recovered = store.snapshot(operator, now(4));
        assert_eq!(recovered.state, LivePositionState::Current);
        assert_eq!(recovered.consecutive_failures, 0);
        assert_eq!(recovered.aircraft_count, 2);
        assert_eq!(recovered.provider, Some("airplanes.live"));
        assert_eq!(recovered.attribution.unwrap().source_name, "Airplanes.live");
        assert!(recovered.last_error_code.is_none());
    }

    #[test]
    fn high_latency_timeout_is_visible_as_degraded_before_unavailable() {
        let store = LivePositionStatusStore::default();
        let operator = OperatorId::new();
        store.register(
            operator,
            LivePositionRegion {
                latitude_degrees: 37.62,
                longitude_degrees: -122.38,
                radius_nautical_miles: 25,
            },
            Duration::from_secs(30),
            now(0),
        );

        store.record_failure(operator, now(1), "timeout");
        let degraded = store.snapshot(operator, now(1));
        assert_eq!(degraded.state, LivePositionState::Degraded);
        assert_eq!(degraded.last_error_code.as_deref(), Some("timeout"));

        store.record_failure(operator, now(2), "timeout");
        store.record_failure(operator, now(3), "timeout");
        assert_eq!(
            store.snapshot(operator, now(3)).state,
            LivePositionState::Unavailable
        );
    }
}
