use std::{env, net::SocketAddr};

use thiserror::Error;

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8080";

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub database_url: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("DATABASE_URL must be set")]
    MissingDatabaseUrl,
    #[error("API_BIND_ADDRESS must be a valid socket address: {0}")]
    InvalidBindAddress(#[from] std::net::AddrParseError),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_address = env::var("API_BIND_ADDRESS")
            .unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_owned())
            .parse()?;
        let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;

        Ok(Self {
            bind_address,
            database_url,
        })
    }
}
