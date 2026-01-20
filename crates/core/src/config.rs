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
