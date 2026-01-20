//! Worker configuration
//!
//! Loads configuration from environment variables

use anyhow::Result;

/// Worker configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub telegram_bot_token: String,
    pub poll_interval_seconds: u64,
    pub batch_size: i64,
    pub max_retry_attempts: i32,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://localhost/televent".to_string()),
            telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN")?,
            poll_interval_seconds: std::env::var("WORKER_POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            batch_size: std::env::var("WORKER_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            max_retry_attempts: std::env::var("WORKER_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        })
    }
}
