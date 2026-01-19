//! Bot configuration
//!
//! Loads configuration from environment variables

use anyhow::Result;

/// Bot configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Telegram bot token
    pub bot_token: String,

    /// Database connection URL
    pub database_url: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN environment variable not set"))?;

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not set"))?;

        Ok(Self {
            bot_token,
            database_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_structure() {
        // Verify Config implements required traits
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}

        assert_clone::<Config>();
        assert_debug::<Config>();
    }
}
