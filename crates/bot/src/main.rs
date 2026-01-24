//! Televent Bot - Telegram bot binary (standalone mode)
//!
//! This binary runs the bot as a standalone service.
//! For library usage, see the bot crate's lib.rs.

mod config;

use anyhow::Result;
use config::Config;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,bot=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Televent Telegram bot (standalone mode)");

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Create database connection pool with explicit configuration
    // Standalone bot mode: sized for Telegram message handlers (~10 connections)
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800)) // 30 minutes
        .connect(&config.database_url)
        .await?;
    tracing::info!("✓ Database pool established (max_connections: 10)");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Run bot using library function
    bot::run_bot(pool, config.telegram_bot_token.clone()).await?;

    Ok(())
}
