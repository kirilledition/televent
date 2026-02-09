//! Televent Worker - Background job processor binary (standalone mode)
//!
//! This binary runs the worker as a standalone service.
//! For library usage, see the worker crate's lib.rs.

use anyhow::Result;
use televent_shared::bootstrap;
use teloxide::Bot;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Init env
    bootstrap::init_env();

    // 2. Init tracing
    let _guard = bootstrap::init_tracing("worker");

    info!("Starting Televent worker (standalone mode)");

    // 3. Load configuration (use library's exported Config)
    let config = worker::Config::from_env()?;

    // 4. Init DB
    let pool = bootstrap::init_db(&config.core).await?;

    // Run migrations
    sqlx::migrate!("../migrations").run(&pool).await?;
    info!("Database migrations completed");

    // Initialize Telegram bot
    let bot = Bot::new(&config.telegram_bot_token);
    info!("Telegram bot initialized");

    // Run worker using library function (no shutdown token in standalone mode)
    worker::run_worker(pool, bot, config, None).await?;

    Ok(())
}
