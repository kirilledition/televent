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

    /// Interval in seconds for logging queue status (COUNT(*))
    pub status_log_interval_secs: u64,

    /// SMTP Host
    pub smtp_host: String,

    /// SMTP Port
    pub smtp_port: u16,

    /// SMTP Username
    pub smtp_username: Option<String>,

    /// SMTP Password
    pub smtp_password: Option<String>,

    /// SMTP From Address
    pub smtp_from: String,

    /// SMTP Connection Pool Size
    pub smtp_pool_size: u32,
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

            status_log_interval_secs: env::var("WORKER_STATUS_LOG_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("WORKER_STATUS_LOG_INTERVAL_SECS must be a valid integer")?,

            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),

            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "1025".to_string())
                .parse()
                .context("SMTP_PORT must be a valid integer")?,

            smtp_username: env::var("SMTP_USERNAME").ok(),

            smtp_password: env::var("SMTP_PASSWORD").ok(),

            smtp_from: env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@televent.app".to_string()),

            smtp_pool_size: env::var("SMTP_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("SMTP_POOL_SIZE must be a valid integer")?,
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
        let config = Config {
            core: CoreConfig {
                database_url: "postgres://localhost".to_string(),
                telegram_bot_token: "test_token".to_string(),
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "noreply@televent.app".to_string(),
            smtp_pool_size: 10,
        };

        assert_eq!(config.poll_interval_secs, 10);
        assert_eq!(config.max_retry_count, 5);
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.smtp_port, 1025);
    }

    #[test]
    fn test_config_deref() {
        let config = Config {
            core: CoreConfig {
                database_url: "postgres://test@localhost/db".to_string(),
                telegram_bot_token: "test_bot_token".to_string(),
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "noreply@televent.app".to_string(),
            smtp_pool_size: 10,
        };

        // Test Deref trait
        assert_eq!(config.database_url, "postgres://test@localhost/db");
        assert_eq!(config.telegram_bot_token, "test_bot_token");
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            core: CoreConfig {
                database_url: "postgres://localhost".to_string(),
                telegram_bot_token: "test_token".to_string(),
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_username: Some("user".to_string()),
            smtp_password: Some("pass".to_string()),
            smtp_from: "noreply@televent.app".to_string(),
            smtp_pool_size: 10,
        };

        let cloned = config.clone();
        assert_eq!(cloned.poll_interval_secs, config.poll_interval_secs);
        assert_eq!(cloned.smtp_username, config.smtp_username);
        assert_eq!(cloned.smtp_password, config.smtp_password);
    }

    #[test]
    fn test_config_debug() {
        let config = Config {
            core: CoreConfig {
                database_url: "postgres://localhost".to_string(),
                telegram_bot_token: "test_token".to_string(),
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "noreply@televent.app".to_string(),
            smtp_pool_size: 10,
        };

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("poll_interval_secs"));
    }
}
