use std::{env, net::SocketAddr, path::PathBuf};

use thiserror::Error;

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8080";

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
pub struct Config {
    pub bind_address: SocketAddr,
    pub database_url: String,
    pub replay: Option<ReplayConfig>,
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

        Ok(Self {
            bind_address,
            database_url,
            replay,
        })
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
}
