//! Televent Worker - Background job processor binary (standalone mode)
//!
//! This binary runs the worker as a standalone service.
//! For library usage, see the worker crate's lib.rs.

use anyhow::Result;
use teloxide::Bot;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,worker=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Televent worker (standalone mode)");

    // Load configuration (use library's exported Config)
    let config = worker::Config::from_env()?;

    // Create database connection pool with explicit configuration
    // Standalone worker mode: sized for job processing (~10 connections)
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800)) // 30 minutes
        .connect(&config.database_url)
        .await?;
    info!("✓ Database pool established (max_connections: 10)");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    info!("Database migrations completed");

    // Initialize Telegram bot
    let bot = Bot::new(&config.telegram_bot_token);
    info!("Telegram bot initialized");

    // Run worker using library function (no shutdown token in standalone mode)
    worker::run_worker(pool, bot, config, None).await?;

    Ok(())
}
