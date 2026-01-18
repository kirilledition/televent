//! Server configuration from environment variables

use anyhow::{Context, Result};
use std::env;

/// Server configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub telegram_bot_token: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("API_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("Failed to parse API_PORT as u16")?,
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL environment variable not set")?,
            jwt_secret: env::var("JWT_SECRET")
                .context("JWT_SECRET environment variable not set")?,
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
                .context("TELEGRAM_BOT_TOKEN environment variable not set")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_from_env_with_defaults() {
        // Set only required env vars
        env::set_var("DATABASE_URL", "postgres://test");
        env::set_var("JWT_SECRET", "test_secret");
        env::set_var("TELEGRAM_BOT_TOKEN", "test_token");

        let config = Config::from_env().unwrap();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.database_url, "postgres://test");
        assert_eq!(config.jwt_secret, "test_secret");
        assert_eq!(config.telegram_bot_token, "test_token");

        // Cleanup
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        env::remove_var("TELEGRAM_BOT_TOKEN");
    }

    #[test]
    fn test_config_from_env_with_custom_values() {
        env::set_var("API_HOST", "127.0.0.1");
        env::set_var("API_PORT", "8080");
        env::set_var("DATABASE_URL", "postgres://custom");
        env::set_var("JWT_SECRET", "custom_secret");
        env::set_var("TELEGRAM_BOT_TOKEN", "custom_token");

        let config = Config::from_env().unwrap();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.database_url, "postgres://custom");

        // Cleanup
        env::remove_var("API_HOST");
        env::remove_var("API_PORT");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        env::remove_var("TELEGRAM_BOT_TOKEN");
    }

    #[test]
    fn test_config_missing_required_vars() {
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        env::remove_var("TELEGRAM_BOT_TOKEN");

        let result = Config::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_invalid_port() {
        env::set_var("API_PORT", "invalid");
        env::set_var("DATABASE_URL", "postgres://test");
        env::set_var("JWT_SECRET", "test_secret");
        env::set_var("TELEGRAM_BOT_TOKEN", "test_token");

        let result = Config::from_env();
        assert!(result.is_err());

        // Cleanup
        env::remove_var("API_PORT");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        env::remove_var("TELEGRAM_BOT_TOKEN");
    }
}
