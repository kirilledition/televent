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
    /// Create config from explicit values
    pub fn new(
        host: String,
        port: u16,
        database_url: String,
        jwt_secret: String,
        telegram_bot_token: String,
    ) -> Self {
        Self {
            host,
            port,
            database_url,
            jwt_secret,
            telegram_bot_token,
        }
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self::new(
            env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            env::var("API_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("Failed to parse API_PORT as u16")?,
            env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?,
            env::var("JWT_SECRET").context("JWT_SECRET environment variable not set")?,
            env::var("TELEGRAM_BOT_TOKEN")
                .context("TELEGRAM_BOT_TOKEN environment variable not set")?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new_with_defaults() {
        let config = Config::new(
            "0.0.0.0".to_string(),
            3000,
            "postgres://test".to_string(),
            "test_secret".to_string(),
            "test_token".to_string(),
        );

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.database_url, "postgres://test");
        assert_eq!(config.jwt_secret, "test_secret");
        assert_eq!(config.telegram_bot_token, "test_token");
    }

    #[test]
    fn test_config_new_with_custom_values() {
        let config = Config::new(
            "127.0.0.1".to_string(),
            8080,
            "postgres://custom".to_string(),
            "custom_secret".to_string(),
            "custom_token".to_string(),
        );

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.database_url, "postgres://custom");
        assert_eq!(config.jwt_secret, "custom_secret");
        assert_eq!(config.telegram_bot_token, "custom_token");
    }

    #[test]
    fn test_config_clone() {
        let config = Config::new(
            "localhost".to_string(),
            9000,
            "postgres://db".to_string(),
            "secret".to_string(),
            "token".to_string(),
        );

        let cloned = config.clone();
        assert_eq!(config.host, cloned.host);
        assert_eq!(config.port, cloned.port);
        assert_eq!(config.database_url, cloned.database_url);
    }

    #[test]
    fn test_config_debug() {
        let config = Config::new(
            "127.0.0.1".to_string(),
            8080,
            "postgres://test".to_string(),
            "secret".to_string(),
            "token".to_string(),
        );

        // Verify Debug trait is implemented
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("127.0.0.1"));
        assert!(debug_str.contains("8080"));
    }
}
