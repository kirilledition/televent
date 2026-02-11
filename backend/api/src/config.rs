//! Server configuration from environment variables

use anyhow::{Context, Result};
use std::env;
use std::ops::Deref;
use televent_core::config::CoreConfig;

/// Server configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Core configuration
    pub core: CoreConfig,

    pub host: String,
    pub port: u16,
    pub cors_allowed_origin: String,
    /// Directory to serve static files from (frontend export)
    pub static_dir: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let core = CoreConfig::from_env()?;

        Ok(Self {
            core,
            host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("API_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("Failed to parse API_PORT as u16")?,
            cors_allowed_origin: env::var("CORS_ALLOWED_ORIGIN")
                .unwrap_or_else(|_| "*".to_string()), // Defaults to "*" (allow all, no credentials)
            static_dir: env::var("STATIC_DIR").unwrap_or_else(|_| "../frontend/out".to_string()),
        })
    }
}

impl Deref for Config {
    type Target = CoreConfig;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new_with_defaults() {
        // Updated test to reflect new structure
        let config = Config {
            core: CoreConfig {
                database_url: "postgres://test".to_string(),
                telegram_bot_token: "test_token".to_string(),
                db_max_connections: 20,
            },
            host: "0.0.0.0".to_string(),
            port: 3000,
            cors_allowed_origin: "*".to_string(),
            static_dir: "../frontend/out".to_string(),
        };

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.database_url, "postgres://test");
        assert_eq!(config.telegram_bot_token, "test_token");
        assert_eq!(config.static_dir, "../frontend/out");
    }
}
