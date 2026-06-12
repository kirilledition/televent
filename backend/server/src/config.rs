use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct UnifiedConfig {
    pub runtime: RuntimeConfig,
    pub api: ApiConfig,
    pub worker: WorkerConfig,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub database_url: String,
    pub telegram_bot_token: String,
    pub db_max_connections: u32,
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub cors_allowed_origin: String,
    pub frontend_static_dir: Option<String>,
    pub enable_swagger: bool,
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub poll_interval_secs: u64,
    pub max_retry_count: i32,
    pub batch_size: i64,
    pub status_log_interval_secs: u64,
}

impl UnifiedConfig {
    pub fn from_env() -> Result<Self> {
        let runtime = RuntimeConfig::from_env()?;
        let app_env = env::var("APP_ENV")
            .or_else(|_| env::var("RUST_ENV"))
            .unwrap_or_else(|_| "development".to_string());
        let is_production = app_env.eq_ignore_ascii_case("production");

        Ok(Self {
            runtime,
            api: ApiConfig {
                host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
                port: env::var("API_PORT")
                    .or_else(|_| env::var("PORT"))
                    .unwrap_or_else(|_| "3000".into())
                    .parse()?,
                cors_allowed_origin: env::var("CORS_ALLOWED_ORIGIN").unwrap_or_else(|_| {
                    env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into())
                }),
                frontend_static_dir: env::var("FRONTEND_STATIC_DIR")
                    .ok()
                    .or_else(|| Some("../frontend/out".into())),
                enable_swagger: parse_env_bool("ENABLE_SWAGGER").unwrap_or(!is_production),
            },
            worker: WorkerConfig {
                poll_interval_secs: env::var("WORKER_POLL_INTERVAL_SECS")
                    .unwrap_or_else(|_| "10".into())
                    .parse()?,
                max_retry_count: env::var("WORKER_MAX_RETRY_COUNT")
                    .unwrap_or_else(|_| "5".into())
                    .parse()?,
                batch_size: env::var("WORKER_BATCH_SIZE")
                    .unwrap_or_else(|_| "10".into())
                    .parse()?,
                status_log_interval_secs: env::var("WORKER_STATUS_LOG_INTERVAL_SECS")
                    .unwrap_or_else(|_| "60".into())
                    .parse()?,
            },
        })
    }

    pub fn to_api_config(&self) -> api::config::Config {
        api::config::Config {
            host: self.api.host.clone(),
            port: self.api.port,
            cors_allowed_origin: self.api.cors_allowed_origin.clone(),
            frontend_static_dir: self.api.frontend_static_dir.clone(),
            enable_swagger: self.api.enable_swagger,
        }
    }

    pub fn to_worker_config(&self) -> worker::Config {
        worker::Config {
            poll_interval_secs: self.worker.poll_interval_secs,
            max_retry_count: self.worker.max_retry_count,
            batch_size: self.worker.batch_size,
            status_log_interval_secs: self.worker.status_log_interval_secs,
        }
    }
}

impl RuntimeConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
                .context("TELEGRAM_BOT_TOKEN must be set")?,
            db_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .context("DATABASE_MAX_CONNECTIONS must be a positive integer")?,
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
