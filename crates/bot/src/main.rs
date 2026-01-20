//! Televent Bot - Telegram bot for calendar management
//!
//! This crate provides Telegram bot functionality for managing calendars.

mod commands;
mod config;
mod db;
mod dialogue;
mod handlers;

use anyhow::Result;
use commands::Command;
use config::Config;
use db::BotDb;
use sqlx::PgPool;
use teloxide::prelude::*;
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

    tracing::info!("Starting Televent Telegram bot");

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Create database connection pool
    let pool = PgPool::connect(&config.database_url).await?;
    tracing::info!("Database connection pool established");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Create database handle for bot
    let bot_db = BotDb::new(pool);

    // Initialize bot
    let bot = Bot::new(&config.bot_token);
    tracing::info!("Bot initialized, starting command handler");

    // Set up command handler
    Command::repl(bot, move |bot: Bot, msg: Message, cmd: Command| {
        let db = bot_db.clone();
        async move {
            let result = handle_command(bot, msg, cmd, db).await;
            if let Err(e) = result {
                tracing::error!("Error handling command: {}", e);
            }
            Ok(())
        }
    })
    .await;

    Ok(())
}

/// Route commands to their handlers
async fn handle_command(bot: Bot, msg: Message, cmd: Command, db: BotDb) -> Result<()> {
    tracing::info!("Handling command: {:?}", cmd);

    match cmd {
        Command::Start => handlers::handle_start(bot, msg, db).await?,
        Command::Help => handlers::handle_help(bot, msg).await?,
        Command::Today => handlers::handle_today(bot, msg, db).await?,
        Command::Tomorrow => handlers::handle_tomorrow(bot, msg, db).await?,
        Command::Week => handlers::handle_week(bot, msg, db).await?,
        Command::Create => handlers::handle_create(bot, msg).await?,
        Command::List => handlers::handle_list(bot, msg).await?,
        Command::Cancel => handlers::handle_cancel(bot, msg).await?,
        Command::Device => handlers::handle_device(bot, msg, db).await?,
        Command::Export => handlers::handle_export(bot, msg, db).await?,
        Command::DeleteAccount => handlers::handle_delete_account(bot, msg).await?,
    }

    Ok(())
}
