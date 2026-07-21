use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

use flight_tracker_api::domain::OperatorId;
use reqwest::Url;
use thiserror::Error;
use uuid::Uuid;

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8080";
const DEFAULT_NOAA_BASE_URL: &str = "https://aviationweather.gov/";
const DEFAULT_NOAA_USER_AGENT: &str =
    "flight-tracker-ai/0.1 (+https://github.com/carlwelchdesign/flight-tracker-ai)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnvironment {
    Development,
    Production,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayConfig {
    pub scenario_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct NoaaConfig {
    pub operator_id: OperatorId,
    pub stations: Vec<String>,
    pub base_url: Url,
    pub user_agent: String,
    pub poll_interval: Duration,
    pub metar_stale_after: Duration,
    pub air_sigmet_stale_after: Duration,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub database_url: String,
    pub replay: Option<ReplayConfig>,
    pub noaa: Option<NoaaConfig>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("DATABASE_URL must be set")]
    MissingDatabaseUrl,
    #[error("API_BIND_ADDRESS must be a valid socket address: {0}")]
    InvalidBindAddress(#[from] std::net::AddrParseError),
    #[error("APP_ENV must be development or production")]
    InvalidAppEnvironment,
    #[error("ENABLE_REPLAY_CONTROLS must be true or false")]
    InvalidReplayToggle,
    #[error("replay controls are forbidden unless APP_ENV=development")]
    ReplayControlsForbidden,
    #[error("REPLAY_SCENARIO_PATH must be set when replay controls are enabled")]
    MissingReplayScenarioPath,
    #[error("ENABLE_NOAA_WEATHER must be true or false")]
    InvalidNoaaToggle,
    #[error("NOAA_OPERATOR_ID must be a UUID when NOAA ingestion is enabled")]
    InvalidNoaaOperator,
    #[error("NOAA_METAR_STATIONS must contain one or more comma-separated ICAO codes")]
    MissingNoaaStations,
    #[error("NOAA_API_BASE_URL must be a valid URL")]
    InvalidNoaaBaseUrl,
    #[error("NOAA_USER_AGENT must identify the application")]
    MissingNoaaUserAgent,
    #[error("NOAA_POLL_INTERVAL_SECONDS must be an integer of at least 60")]
    InvalidNoaaPollInterval,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let bind_address = lookup("API_BIND_ADDRESS")
            .unwrap_or_else(|| DEFAULT_BIND_ADDRESS.to_owned())
            .parse()?;
        let database_url = lookup("DATABASE_URL").ok_or(ConfigError::MissingDatabaseUrl)?;
        let environment = match lookup("APP_ENV").as_deref().unwrap_or("production") {
            "development" => AppEnvironment::Development,
            "production" => AppEnvironment::Production,
            _ => return Err(ConfigError::InvalidAppEnvironment),
        };
        let replay_enabled = match lookup("ENABLE_REPLAY_CONTROLS")
            .as_deref()
            .unwrap_or("false")
        {
            "true" => true,
            "false" => false,
            _ => return Err(ConfigError::InvalidReplayToggle),
        };
        let replay = if replay_enabled {
            if environment != AppEnvironment::Development {
                return Err(ConfigError::ReplayControlsForbidden);
            }
            let scenario_path = lookup("REPLAY_SCENARIO_PATH")
                .filter(|value| !value.trim().is_empty())
                .ok_or(ConfigError::MissingReplayScenarioPath)?;
            Some(ReplayConfig {
                scenario_path: scenario_path.into(),
            })
        } else {
            None
        };
        let noaa_enabled = parse_bool(
            lookup("ENABLE_NOAA_WEATHER").as_deref().unwrap_or("false"),
            ConfigError::InvalidNoaaToggle,
        )?;
        let noaa = if noaa_enabled {
            let operator_id = lookup("NOAA_OPERATOR_ID")
                .and_then(|value| Uuid::parse_str(&value).ok())
                .map(OperatorId::from_uuid)
                .ok_or(ConfigError::InvalidNoaaOperator)?;
            let stations = lookup("NOAA_METAR_STATIONS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_ascii_uppercase)
                .collect::<Vec<_>>();
            if stations.is_empty() {
                return Err(ConfigError::MissingNoaaStations);
            }
            let base_url = Url::parse(
                lookup("NOAA_API_BASE_URL")
                    .as_deref()
                    .unwrap_or(DEFAULT_NOAA_BASE_URL),
            )
            .map_err(|_| ConfigError::InvalidNoaaBaseUrl)?;
            let user_agent =
                lookup("NOAA_USER_AGENT").unwrap_or_else(|| DEFAULT_NOAA_USER_AGENT.into());
            if user_agent.trim().is_empty() {
                return Err(ConfigError::MissingNoaaUserAgent);
            }
            let poll_interval_seconds = lookup("NOAA_POLL_INTERVAL_SECONDS")
                .as_deref()
                .unwrap_or("60")
                .parse::<u64>()
                .ok()
                .filter(|value| *value >= 60)
                .ok_or(ConfigError::InvalidNoaaPollInterval)?;
            Some(NoaaConfig {
                operator_id,
                stations,
                base_url,
                user_agent,
                poll_interval: Duration::from_secs(poll_interval_seconds),
                metar_stale_after: Duration::from_secs(15 * 60),
                air_sigmet_stale_after: Duration::from_secs(3 * 60),
            })
        } else {
            None
        };

        Ok(Self {
            bind_address,
            database_url,
            replay,
            noaa,
        })
    }
}

fn parse_bool(value: &str, error: ConfigError) -> Result<bool, ConfigError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config(values: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let mut environment = HashMap::from([(
            "DATABASE_URL".to_owned(),
            "postgres://example.invalid/database".to_owned(),
        )]);
        environment.extend(
            values
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned())),
        );
        Config::from_lookup(|key| environment.get(key).cloned())
    }

    #[test]
    fn replay_is_disabled_by_default() {
        assert!(config(&[]).unwrap().replay.is_none());
    }

    #[test]
    fn production_cannot_enable_replay_controls() {
        let error = config(&[
            ("APP_ENV", "production"),
            ("ENABLE_REPLAY_CONTROLS", "true"),
            ("REPLAY_SCENARIO_PATH", "fixture.json"),
        ])
        .unwrap_err();
        assert!(matches!(error, ConfigError::ReplayControlsForbidden));
    }

    #[test]
    fn development_requires_and_accepts_a_scenario_path() {
        assert!(matches!(
            config(&[
                ("APP_ENV", "development"),
                ("ENABLE_REPLAY_CONTROLS", "true")
            ]),
            Err(ConfigError::MissingReplayScenarioPath)
        ));
        assert!(
            config(&[
                ("APP_ENV", "development"),
                ("ENABLE_REPLAY_CONTROLS", "true"),
                ("REPLAY_SCENARIO_PATH", "fixture.json"),
            ])
            .unwrap()
            .replay
            .is_some()
        );
    }

    #[test]
    fn noaa_ingestion_is_disabled_by_default_and_validated_when_enabled() {
        assert!(config(&[]).unwrap().noaa.is_none());
        assert!(matches!(
            config(&[("ENABLE_NOAA_WEATHER", "true")]),
            Err(ConfigError::InvalidNoaaOperator)
        ));
        let configured = config(&[
            ("ENABLE_NOAA_WEATHER", "true"),
            ("NOAA_OPERATOR_ID", "00000000-0000-0000-0000-000000000001"),
            ("NOAA_METAR_STATIONS", "ksfo, koak"),
        ])
        .unwrap()
        .noaa
        .unwrap();
        assert_eq!(configured.stations, vec!["KSFO", "KOAK"]);
        assert_eq!(configured.poll_interval, Duration::from_secs(60));
    }

    #[test]
    fn noaa_poll_interval_cannot_violate_provider_rate_discipline() {
        let error = config(&[
            ("ENABLE_NOAA_WEATHER", "true"),
            ("NOAA_OPERATOR_ID", "00000000-0000-0000-0000-000000000001"),
            ("NOAA_METAR_STATIONS", "KSFO"),
            ("NOAA_POLL_INTERVAL_SECONDS", "59"),
        ])
        .unwrap_err();
        assert!(matches!(error, ConfigError::InvalidNoaaPollInterval));
    }
}
