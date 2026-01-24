use anyhow::Result;
use api::AppState;
use moka::future::Cache;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Create file appender
    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,api=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!("Starting Televent API server");

    // Load configuration
    let config = api::config::Config::from_env()?;
    tracing::info!(
        "Server configuration loaded: {}:{}",
        config.host,
        config.port
    );

    // Create database connection pool with explicit configuration
    // Standalone API mode: sized for API requests only (~20 connections)
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800)) // 30 minutes
        .connect(&config.database_url)
        .await?;
    tracing::info!("✓ Database pool established (max_connections: 20)");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Initialize Auth Cache (TTL: 5 minutes)
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState { pool, auth_cache };

    // Start server using library function
    api::run_api(state, &config).await?;

    Ok(())
}
