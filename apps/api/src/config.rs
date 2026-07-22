use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

use flight_tracker_api::{
    auth::{AssertionConfig, AssertionKey, AuthRole, DevelopmentIdentity},
    domain::OperatorId,
    live_positions::LivePositionRegion,
};
use reqwest::Url;
use thiserror::Error;
use uuid::Uuid;

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8080";
const DEFAULT_NOAA_BASE_URL: &str = "https://aviationweather.gov/";
const DEFAULT_NOAA_USER_AGENT: &str =
    "flight-tracker-ai/0.1 (+https://github.com/carlwelchdesign/flight-tracker-ai)";
const DEFAULT_ADSB_LOL_BASE_URL: &str = "https://api.adsb.lol/";
const DEFAULT_ADSB_LOL_USER_AGENT: &str =
    "flight-tracker-ai/0.1 (+https://github.com/carlwelchdesign/flight-tracker-ai)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnvironment {
    Development,
    Production,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayConfig {
    Development { scenario_path: PathBuf },
    Portfolio,
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
pub struct AdsbLolConfig {
    pub operator_id: OperatorId,
    pub region: LivePositionRegion,
    pub base_url: Url,
    pub user_agent: String,
    pub poll_interval: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    Development,
    Clerk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub assertion: AssertionConfig,
    pub development_identity: Option<DevelopmentIdentity>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub database_url: String,
    pub replay: Option<ReplayConfig>,
    pub noaa: Option<NoaaConfig>,
    pub public_weather_operator: Option<OperatorId>,
    pub adsb_lol: Option<AdsbLolConfig>,
    pub auth: AuthConfig,
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
    #[error("ENABLE_PORTFOLIO_REPLAY must be true or false")]
    InvalidPortfolioReplayToggle,
    #[error("development and portfolio replay modes cannot both be enabled")]
    ConflictingReplayModes,
    #[error("portfolio replay is forbidden unless APP_ENV=production")]
    PortfolioReplayForbidden,
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
    #[error("PUBLIC_WEATHER_OPERATOR_ID must be a UUID when configured")]
    InvalidPublicWeatherOperator,
    #[error("ENABLE_ADSB_LOL_POSITIONS must be true or false")]
    InvalidAdsbLolToggle,
    #[error("ADSB_LOL_OPERATOR_ID must be a UUID when ADSB.lol ingestion is enabled")]
    InvalidAdsbLolOperator,
    #[error("ADSB_LOL_LATITUDE and ADSB_LOL_LONGITUDE must be finite WGS84 coordinates")]
    InvalidAdsbLolCenter,
    #[error("ADSB_LOL_RADIUS_NM must be an integer from 1 through 100")]
    InvalidAdsbLolRadius,
    #[error("ADSB_LOL_POLL_INTERVAL_SECONDS must be an integer of at least 30")]
    InvalidAdsbLolPollInterval,
    #[error("ADSB_LOL_API_BASE_URL must be a valid HTTP or HTTPS URL")]
    InvalidAdsbLolBaseUrl,
    #[error("ADSB_LOL_USER_AGENT must identify the application")]
    MissingAdsbLolUserAgent,
    #[error("AUTH_MODE must be development or clerk")]
    InvalidAuthMode,
    #[error("AUTH_MODE=development is forbidden unless APP_ENV=development")]
    DevelopmentAuthForbidden,
    #[error("INTERNAL_AUTH_SECRET must contain at least 32 bytes")]
    InvalidInternalAuthSecret,
    #[error("INTERNAL_AUTH_KEY_ID must be configured")]
    MissingInternalAuthKeyId,
    #[error(
        "INTERNAL_AUTH_PREVIOUS_KEY_ID and INTERNAL_AUTH_PREVIOUS_SECRET must be configured together"
    )]
    IncompletePreviousInternalAuthKey,
    #[error("AUTH_ASSERTION_ISSUER and AUTH_ASSERTION_AUDIENCE must not be empty")]
    InvalidAuthBoundary,
    #[error("development auth requires DEV_AUTH_OPERATOR_ID to be a UUID")]
    InvalidDevelopmentOperator,
    #[error("development auth requires non-empty DEV_AUTH_TENANT_ID and DEV_AUTH_SUBJECT")]
    InvalidDevelopmentIdentity,
    #[error("DEV_AUTH_ROLE must be viewer, dispatcher, operator, or administrator")]
    InvalidDevelopmentRole,
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
        let portfolio_replay_enabled = parse_bool(
            lookup("ENABLE_PORTFOLIO_REPLAY")
                .as_deref()
                .unwrap_or("false"),
            ConfigError::InvalidPortfolioReplayToggle,
        )?;
        if replay_enabled && portfolio_replay_enabled {
            return Err(ConfigError::ConflictingReplayModes);
        }
        let replay = if replay_enabled {
            if environment != AppEnvironment::Development {
                return Err(ConfigError::ReplayControlsForbidden);
            }
            let scenario_path = lookup("REPLAY_SCENARIO_PATH")
                .filter(|value| !value.trim().is_empty())
                .ok_or(ConfigError::MissingReplayScenarioPath)?;
            Some(ReplayConfig::Development {
                scenario_path: scenario_path.into(),
            })
        } else if portfolio_replay_enabled {
            if environment != AppEnvironment::Production {
                return Err(ConfigError::PortfolioReplayForbidden);
            }
            Some(ReplayConfig::Portfolio)
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
        let public_weather_operator = match lookup("PUBLIC_WEATHER_OPERATOR_ID") {
            Some(value) => Some(
                Uuid::parse_str(&value)
                    .map(OperatorId::from_uuid)
                    .map_err(|_| ConfigError::InvalidPublicWeatherOperator)?,
            ),
            None => noaa.as_ref().map(|value| value.operator_id),
        };
        let adsb_lol_enabled = parse_bool(
            lookup("ENABLE_ADSB_LOL_POSITIONS")
                .as_deref()
                .unwrap_or("false"),
            ConfigError::InvalidAdsbLolToggle,
        )?;
        let adsb_lol = if adsb_lol_enabled {
            let operator_id = lookup("ADSB_LOL_OPERATOR_ID")
                .and_then(|value| Uuid::parse_str(&value).ok())
                .map(OperatorId::from_uuid)
                .ok_or(ConfigError::InvalidAdsbLolOperator)?;
            let latitude_degrees = lookup("ADSB_LOL_LATITUDE")
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite() && (-90.0..=90.0).contains(value))
                .ok_or(ConfigError::InvalidAdsbLolCenter)?;
            let longitude_degrees = lookup("ADSB_LOL_LONGITUDE")
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite() && (-180.0..=180.0).contains(value))
                .ok_or(ConfigError::InvalidAdsbLolCenter)?;
            let radius_nautical_miles = lookup("ADSB_LOL_RADIUS_NM")
                .as_deref()
                .unwrap_or("25")
                .parse::<u16>()
                .ok()
                .filter(|value| (1..=100).contains(value))
                .ok_or(ConfigError::InvalidAdsbLolRadius)?;
            let poll_interval_seconds = lookup("ADSB_LOL_POLL_INTERVAL_SECONDS")
                .as_deref()
                .unwrap_or("30")
                .parse::<u64>()
                .ok()
                .filter(|value| *value >= 30)
                .ok_or(ConfigError::InvalidAdsbLolPollInterval)?;
            let base_url = Url::parse(
                lookup("ADSB_LOL_API_BASE_URL")
                    .as_deref()
                    .unwrap_or(DEFAULT_ADSB_LOL_BASE_URL),
            )
            .ok()
            .filter(|url| matches!(url.scheme(), "http" | "https"))
            .ok_or(ConfigError::InvalidAdsbLolBaseUrl)?;
            let user_agent =
                lookup("ADSB_LOL_USER_AGENT").unwrap_or_else(|| DEFAULT_ADSB_LOL_USER_AGENT.into());
            if user_agent.trim().is_empty() {
                return Err(ConfigError::MissingAdsbLolUserAgent);
            }
            Some(AdsbLolConfig {
                operator_id,
                region: LivePositionRegion {
                    latitude_degrees,
                    longitude_degrees,
                    radius_nautical_miles,
                },
                base_url,
                user_agent,
                poll_interval: Duration::from_secs(poll_interval_seconds),
            })
        } else {
            None
        };

        let auth_mode = match lookup("AUTH_MODE").as_deref() {
            Some("development") => AuthMode::Development,
            Some("clerk") => AuthMode::Clerk,
            _ => return Err(ConfigError::InvalidAuthMode),
        };
        if auth_mode == AuthMode::Development && environment != AppEnvironment::Development {
            return Err(ConfigError::DevelopmentAuthForbidden);
        }
        let secret = lookup("INTERNAL_AUTH_SECRET").unwrap_or_default();
        if secret.len() < 32 {
            return Err(ConfigError::InvalidInternalAuthSecret);
        }
        let key_id = lookup("INTERNAL_AUTH_KEY_ID")
            .filter(|value| !value.trim().is_empty())
            .ok_or(ConfigError::MissingInternalAuthKeyId)?;
        let previous_key = match (
            lookup("INTERNAL_AUTH_PREVIOUS_KEY_ID").filter(|value| !value.trim().is_empty()),
            lookup("INTERNAL_AUTH_PREVIOUS_SECRET").filter(|value| !value.is_empty()),
        ) {
            (None, None) => None,
            (Some(id), Some(secret)) => Some(AssertionKey { id, secret }),
            _ => return Err(ConfigError::IncompletePreviousInternalAuthKey),
        };
        let issuer = lookup("AUTH_ASSERTION_ISSUER").unwrap_or_else(|| "flight-tracker-web".into());
        let audience =
            lookup("AUTH_ASSERTION_AUDIENCE").unwrap_or_else(|| "flight-tracker-api".into());
        if issuer.trim().is_empty() || audience.trim().is_empty() {
            return Err(ConfigError::InvalidAuthBoundary);
        }
        let development_identity = if auth_mode == AuthMode::Development {
            let operator_id = lookup("DEV_AUTH_OPERATOR_ID")
                .and_then(|value| Uuid::parse_str(&value).ok())
                .map(OperatorId::from_uuid)
                .ok_or(ConfigError::InvalidDevelopmentOperator)?;
            let external_tenant_id = lookup("DEV_AUTH_TENANT_ID").unwrap_or_default();
            let subject = lookup("DEV_AUTH_SUBJECT").unwrap_or_default();
            if external_tenant_id.trim().is_empty() || subject.trim().is_empty() {
                return Err(ConfigError::InvalidDevelopmentIdentity);
            }
            let role = match lookup("DEV_AUTH_ROLE").as_deref() {
                Some("viewer") => AuthRole::Viewer,
                Some("dispatcher") => AuthRole::Dispatcher,
                Some("operator") => AuthRole::Operator,
                Some("administrator") => AuthRole::Administrator,
                _ => return Err(ConfigError::InvalidDevelopmentRole),
            };
            Some(DevelopmentIdentity {
                operator_id,
                operator_code: lookup("DEV_AUTH_OPERATOR_CODE").unwrap_or_else(|| "SIM".into()),
                operator_name: lookup("DEV_AUTH_OPERATOR_NAME")
                    .unwrap_or_else(|| "Simulation Operator".into()),
                external_tenant_id,
                subject,
                display_name: lookup("DEV_AUTH_DISPLAY_NAME")
                    .unwrap_or_else(|| "Local Administrator".into()),
                role,
            })
        } else {
            None
        };

        Ok(Self {
            bind_address,
            database_url,
            replay,
            noaa,
            public_weather_operator,
            adsb_lol,
            auth: AuthConfig {
                mode: auth_mode,
                assertion: AssertionConfig {
                    active_key: AssertionKey { id: key_id, secret },
                    previous_key,
                    issuer,
                    audience,
                    leeway_seconds: 5,
                },
                development_identity,
            },
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
        let mut environment = HashMap::from([
            (
                "DATABASE_URL".to_owned(),
                "postgres://example.invalid/database".to_owned(),
            ),
            ("APP_ENV".to_owned(), "development".to_owned()),
            ("AUTH_MODE".to_owned(), "development".to_owned()),
            (
                "INTERNAL_AUTH_SECRET".to_owned(),
                "development-only-secret-at-least-32-bytes".to_owned(),
            ),
            (
                "INTERNAL_AUTH_KEY_ID".to_owned(),
                "local-primary".to_owned(),
            ),
            (
                "DEV_AUTH_OPERATOR_ID".to_owned(),
                "9c704a09-a62c-43d5-bac6-94ea2fd53b32".to_owned(),
            ),
            (
                "DEV_AUTH_TENANT_ID".to_owned(),
                "local-flight-tracker".to_owned(),
            ),
            ("DEV_AUTH_SUBJECT".to_owned(), "local-admin".to_owned()),
            ("DEV_AUTH_ROLE".to_owned(), "administrator".to_owned()),
        ]);
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
    fn production_accepts_only_the_built_in_portfolio_replay() {
        let configured = config(&[
            ("APP_ENV", "production"),
            ("AUTH_MODE", "clerk"),
            ("ENABLE_PORTFOLIO_REPLAY", "true"),
        ])
        .unwrap();
        assert_eq!(configured.replay, Some(ReplayConfig::Portfolio));

        assert!(matches!(
            config(&[("ENABLE_PORTFOLIO_REPLAY", "true")]),
            Err(ConfigError::PortfolioReplayForbidden)
        ));
        assert!(matches!(
            config(&[
                ("ENABLE_REPLAY_CONTROLS", "true"),
                ("ENABLE_PORTFOLIO_REPLAY", "true"),
                ("REPLAY_SCENARIO_PATH", "fixture.json"),
            ]),
            Err(ConfigError::ConflictingReplayModes)
        ));
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

    #[test]
    fn public_weather_operator_can_be_configured_without_starting_ingestion() {
        let configured = config(&[(
            "PUBLIC_WEATHER_OPERATOR_ID",
            "00000000-0000-0000-0000-000000000001",
        )])
        .unwrap();
        assert_eq!(
            configured.public_weather_operator,
            Some(OperatorId::from_uuid(
                Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
            ))
        );
        assert!(matches!(
            config(&[("PUBLIC_WEATHER_OPERATOR_ID", "not-a-uuid")]),
            Err(ConfigError::InvalidPublicWeatherOperator)
        ));
    }

    #[test]
    fn adsb_lol_is_disabled_by_default_and_requires_an_explicit_bounded_region() {
        assert!(config(&[]).unwrap().adsb_lol.is_none());
        assert!(matches!(
            config(&[("ENABLE_ADSB_LOL_POSITIONS", "true")]),
            Err(ConfigError::InvalidAdsbLolOperator)
        ));
        let configured = config(&[
            ("ENABLE_ADSB_LOL_POSITIONS", "true"),
            (
                "ADSB_LOL_OPERATOR_ID",
                "00000000-0000-0000-0000-000000000001",
            ),
            ("ADSB_LOL_LATITUDE", "37.62"),
            ("ADSB_LOL_LONGITUDE", "-122.38"),
        ])
        .unwrap()
        .adsb_lol
        .unwrap();
        assert_eq!(configured.region.radius_nautical_miles, 25);
        assert_eq!(configured.poll_interval, Duration::from_secs(30));
    }

    #[test]
    fn adsb_lol_cannot_exceed_region_or_polling_limits() {
        let required = [
            ("ENABLE_ADSB_LOL_POSITIONS", "true"),
            (
                "ADSB_LOL_OPERATOR_ID",
                "00000000-0000-0000-0000-000000000001",
            ),
            ("ADSB_LOL_LATITUDE", "37.62"),
            ("ADSB_LOL_LONGITUDE", "-122.38"),
        ];
        let mut fast = required.to_vec();
        fast.push(("ADSB_LOL_POLL_INTERVAL_SECONDS", "29"));
        assert!(matches!(
            config(&fast),
            Err(ConfigError::InvalidAdsbLolPollInterval)
        ));
        let mut broad = required.to_vec();
        broad.push(("ADSB_LOL_RADIUS_NM", "101"));
        assert!(matches!(
            config(&broad),
            Err(ConfigError::InvalidAdsbLolRadius)
        ));
    }

    #[test]
    fn development_identity_cannot_be_enabled_in_production() {
        assert!(matches!(
            config(&[("APP_ENV", "production")]),
            Err(ConfigError::DevelopmentAuthForbidden)
        ));
    }

    #[test]
    fn production_accepts_clerk_mode_without_a_development_identity() {
        let configured = config(&[("APP_ENV", "production"), ("AUTH_MODE", "clerk")]).unwrap();
        assert_eq!(configured.auth.mode, AuthMode::Clerk);
        assert!(configured.auth.development_identity.is_none());
    }

    #[test]
    fn internal_assertion_secret_is_never_optional_or_weak() {
        assert!(matches!(
            config(&[("INTERNAL_AUTH_SECRET", "short")]),
            Err(ConfigError::InvalidInternalAuthSecret)
        ));
        assert!(matches!(
            config(&[("INTERNAL_AUTH_KEY_ID", "")]),
            Err(ConfigError::MissingInternalAuthKeyId)
        ));
    }

    #[test]
    fn previous_internal_assertion_key_is_an_explicit_pair() {
        assert!(matches!(
            config(&[("INTERNAL_AUTH_PREVIOUS_KEY_ID", "previous")]),
            Err(ConfigError::IncompletePreviousInternalAuthKey)
        ));
        assert!(matches!(
            config(&[(
                "INTERNAL_AUTH_PREVIOUS_SECRET",
                "previous-secret-at-least-thirty-two-bytes"
            )]),
            Err(ConfigError::IncompletePreviousInternalAuthKey)
        ));

        let configured = config(&[
            ("INTERNAL_AUTH_PREVIOUS_KEY_ID", "previous"),
            (
                "INTERNAL_AUTH_PREVIOUS_SECRET",
                "previous-secret-at-least-thirty-two-bytes",
            ),
        ])
        .unwrap();
        assert_eq!(
            configured.auth.assertion.previous_key,
            Some(AssertionKey {
                id: "previous".into(),
                secret: "previous-secret-at-least-thirty-two-bytes".into(),
            })
        );
    }
}
