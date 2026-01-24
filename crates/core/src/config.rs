//! Shared configuration logic
//!
//! Handles loading of common environment variables.

use anyhow::{Context, Result};
use std::env;

/// Common configuration used across services
#[derive(Debug, Clone)]
pub struct CoreConfig {
    /// Database connection URL
    pub database_url: String,

    /// Telegram bot token
    pub telegram_bot_token: String,
}

impl CoreConfig {
    /// Load common configuration from environment variables
    ///
    /// This will also initialize dotenv if it hasn't been done yet.
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
                .context("TELEGRAM_BOT_TOKEN must be set")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_core_config_from_env() {
        // Set env vars
        unsafe {
            env::set_var("DATABASE_URL", "postgres://test:test@localhost:5432/test");
            env::set_var("TELEGRAM_BOT_TOKEN", "test_token");
        }

        let config = CoreConfig::from_env();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(
            config.database_url,
            "postgres://test:test@localhost:5432/test"
        );
        assert_eq!(config.telegram_bot_token, "test_token");

        // Clean up
        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("TELEGRAM_BOT_TOKEN");
        }
    }
}
