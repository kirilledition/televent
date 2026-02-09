//! Televent Bot - Telegram bot binary (standalone mode)
//!
//! This binary runs the bot as a standalone service.
//! For library usage, see the bot crate's lib.rs.

mod config;

use anyhow::Result;
use config::Config;
use televent_shared::bootstrap;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Init env
    bootstrap::init_env();

    // 2. Init tracing
    let _guard = bootstrap::init_tracing("bot");

    tracing::info!("Starting Televent Telegram bot (standalone mode)");

    // 3. Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // 4. Init DB
    let pool = bootstrap::init_db(&config.core).await?;

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Run bot using library function
    bot::run_bot(pool, config.telegram_bot_token.clone()).await?;

    Ok(())
}
