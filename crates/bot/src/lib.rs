//! Televent Bot - Telegram bot for calendar management
//!
//! This crate provides Telegram bot functionality for managing calendars.

mod commands;
mod config;
pub mod db;
mod event_parser;
mod handlers;

use anyhow::Result;
use commands::Command;
use db::BotDb;
use sqlx::PgPool;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt, UpdateHandler};
use teloxide::dptree;
use teloxide::prelude::*;
use teloxide::RequestError;

/// Build the message handler tree
///
/// This function creates the dispatcher handler tree that routes messages to appropriate handlers.
/// Extracted into a separate function to enable testing with teloxide_tests.
pub fn build_handler_tree() -> UpdateHandler<RequestError> {
    Update::filter_message()
        // First try to handle as a command
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        // Then handle as text message (for event creation)
        .branch(dptree::filter(|msg: Message| msg.text().is_some()).endpoint(handle_message))
}

/// Run the Telegram bot service
///
/// This function initializes the bot dispatcher and runs until it exits or encounters an error.
/// It does not handle Ctrl+C signals - that should be handled by the caller.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `bot_token` - Telegram bot token for authentication
pub async fn run_bot(pool: PgPool, bot_token: String) -> Result<()> {
    // Create database handle for bot
    let bot_db = BotDb::new(pool);

    // Initialize bot
    let bot = Bot::new(bot_token);
    tracing::info!("Bot initialized, starting dispatcher");

    // Create dispatcher with database dependency
    // Note: NOT using enable_ctrlc_handler() - shutdown is managed by the caller
    Dispatcher::builder(bot, build_handler_tree())
        .dependencies(dptree::deps![bot_db])
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
        Command::List => handlers::handle_list(bot, msg, db).await,
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
