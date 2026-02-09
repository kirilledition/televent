use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use televent_core::config::CoreConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize dotenvy
pub fn init_env() {
    dotenvy::dotenv().ok();
}

/// Initialize tracing with optional file logging
pub fn init_tracing(service_name: &str) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    // If LOG_DIR is set, use it. Default to "logs/app"
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs/app".to_string());

    let now = chrono::Local::now().format("%y-%m-%d-%H-%M-%S").to_string();
    let filename = format!("televent-{}.log.{}.jsonl", service_name, now);

    let file_appender = tracing_appender::rolling::never(&log_dir, filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let default_filter = format!("info,{}=debug,sqlx=warn", service_name);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .init();

    Some(guard)
}

/// Initialize database pool
pub async fn init_db(config: &CoreConfig) -> Result<sqlx::PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800)) // 30 minutes
        .connect(&config.database_url)
        .await?;

    tracing::info!("✓ Database pool established (max_connections: {})", config.db_max_connections);

    Ok(pool)
}
