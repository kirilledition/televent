use anyhow::Result;
use std::env;
use televent_core::config::CoreConfig;

#[derive(Debug, Clone)]
pub struct UnifiedConfig {
    pub core: CoreConfig,
    pub api: ApiConfig,
    pub worker: WorkerConfig,
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub cors_allowed_origin: String,
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub poll_interval_secs: u64,
    pub max_retry_count: i32,
    pub batch_size: i64,
}

impl UnifiedConfig {
    pub fn from_env() -> Result<Self> {
        let core = CoreConfig::from_env()?;

        Ok(Self {
            core: core.clone(),
            api: ApiConfig {
                host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
                port: env::var("API_PORT")
                    .unwrap_or_else(|_| "3000".into())
                    .parse()?,
                jwt_secret: env::var("JWT_SECRET")?,
                cors_allowed_origin: env::var("CORS_ALLOWED_ORIGIN")
                    .unwrap_or_else(|_| "*".into()),
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
            },
        })
    }

    pub fn to_api_config(&self) -> api::config::Config {
        api::config::Config {
            core: self.core.clone(),
            host: self.api.host.clone(),
            port: self.api.port,
            jwt_secret: self.api.jwt_secret.clone(),
            cors_allowed_origin: self.api.cors_allowed_origin.clone(),
        }
    }

    pub fn to_worker_config(&self) -> worker::Config {
        worker::Config {
            core: self.core.clone(),
            poll_interval_secs: self.worker.poll_interval_secs,
            max_retry_count: self.worker.max_retry_count,
            batch_size: self.worker.batch_size,
        }
    }
}
