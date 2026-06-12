//! Server configuration from environment variables

use anyhow::{Context, Result};
use std::env;

/// Server configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub cors_allowed_origin: String,
    pub frontend_static_dir: Option<String>,
    pub enable_swagger: bool,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let app_env = env::var("APP_ENV")
            .or_else(|_| env::var("RUST_ENV"))
            .unwrap_or_else(|_| "development".to_string());
        let is_production = app_env.eq_ignore_ascii_case("production");

        Ok(Self {
            host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("API_PORT")
                .or_else(|_| env::var("PORT"))
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("Failed to parse API_PORT/PORT as u16")?,
            cors_allowed_origin: env::var("CORS_ALLOWED_ORIGIN").unwrap_or_else(|_| {
                env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
            }),
            frontend_static_dir: env::var("FRONTEND_STATIC_DIR")
                .ok()
                .or_else(|| Some("../frontend/out".to_string())),
            enable_swagger: parse_env_bool("ENABLE_SWAGGER").unwrap_or(!is_production),
        })
    }
}

fn parse_env_bool(name: &str) -> Option<bool> {
    env::var(name).ok().map(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new_with_defaults() {
        let config = Config {
            host: "0.0.0.0".to_string(),
            port: 3000,
            cors_allowed_origin: "http://localhost:3000".to_string(),
            frontend_static_dir: Some("../frontend/out".to_string()),
            enable_swagger: true,
        };

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.cors_allowed_origin, "http://localhost:3000");
    }
}
