//! Televent Bot - Telegram bot for calendar management
//!
//! This crate provides Telegram bot functionality for managing calendars.

mod commands;
mod config;
mod db;
mod event_parser;
mod handlers;

use anyhow::Result;
use commands::Command;
use config::Config;
use db::BotDb;
use sqlx::PgPool;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::dptree;
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
    let bot = Bot::new(&config.telegram_bot_token);
    tracing::info!("Bot initialized, starting dispatcher");

    // Build the message handler schema
    let handler = Update::filter_message()
        // First try to handle as a command
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        // Then handle as text message (for event creation)
        .branch(
            dptree::filter(|msg: Message| msg.text().is_some()).endpoint(handle_message),
        );

    // Create dispatcher with database dependency
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![bot_db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

/// Route commands to their handlers
async fn handle_command(bot: Bot, msg: Message, cmd: Command, db: BotDb) -> ResponseResult<()> {
    tracing::info!("Handling command: {:?}", cmd);

    let result = match cmd {
        Command::Start => handlers::handle_start(bot, msg, db).await,
        Command::Help => handlers::handle_help(bot, msg).await,
        Command::Today => handlers::handle_today(bot, msg, db).await,
        Command::Tomorrow => handlers::handle_tomorrow(bot, msg, db).await,
        Command::Week => handlers::handle_week(bot, msg, db).await,
        Command::Create => handlers::handle_create(bot, msg).await,
        Command::List => handlers::handle_list(bot, msg).await,
        Command::Cancel => handlers::handle_cancel(bot, msg).await,
        Command::Device => handlers::handle_device(bot, msg, db).await,
        Command::Export => handlers::handle_export(bot, msg, db).await,
        Command::Invite => handlers::handle_invite(bot, msg, db).await,
        Command::Rsvp => handlers::handle_rsvp(bot, msg, db).await,
        Command::DeleteAccount => handlers::handle_delete_account(bot, msg).await,
    };

    if let Err(e) = result {
        tracing::error!("Error handling command: {}", e);
    }

    Ok(())
}

/// Handle non-command text messages (event creation)
async fn handle_message(bot: Bot, msg: Message, db: BotDb) -> ResponseResult<()> {
    let result = handlers::handle_text_message(bot, msg, db).await;

    if let Err(e) = result {
        tracing::error!("Error handling text message: {}", e);
    }

    Ok(())
}
