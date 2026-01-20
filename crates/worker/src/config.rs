//! Configuration for worker process
//!
//! Loads configuration from environment variables

use anyhow::{Context, Result};
use std::env;
use std::ops::Deref;
use televent_core::config::CoreConfig;

/// Worker configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Core configuration
    pub core: CoreConfig,

    /// Poll interval in seconds
    pub poll_interval_secs: u64,

    /// Maximum retry count for failed jobs
    pub max_retry_count: i32,

    /// Batch size for processing jobs
    pub batch_size: i64,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let core = CoreConfig::from_env()?;

        Ok(Self {
            core,
            poll_interval_secs: env::var("WORKER_POLL_INTERVAL_SECS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("WORKER_POLL_INTERVAL_SECS must be a valid integer")?,

            max_retry_count: env::var("WORKER_MAX_RETRY_COUNT")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .context("WORKER_MAX_RETRY_COUNT must be a valid integer")?,

            batch_size: env::var("WORKER_BATCH_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("WORKER_BATCH_SIZE must be a valid integer")?,
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
    fn test_config_has_defaults() {
        // Just verify the structure exists and can be created
        // Actual env var tests would require integration tests
        let _ = Config {
            core: CoreConfig {
                database_url: "postgres://localhost".to_string(),
                telegram_bot_token: "test_token".to_string(),
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
        };
    }
}
