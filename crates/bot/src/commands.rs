//! Bot command definitions
//!
//! Defines all Telegram bot commands and their parsing logic

use teloxide::utils::command::BotCommands;

/// All bot commands
#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "Televent calendar bot commands:"
)]
pub enum Command {
    #[command(description = "Start the bot and see welcome message")]
    Start,

    #[command(description = "Show today's events")]
    Today,

    #[command(description = "Show tomorrow's events")]
    Tomorrow,

    #[command(description = "Show this week's events")]
    Week,

    #[command(description = "Create a new event (interactive)")]
    Create,

    #[command(description = "List all events in a date range")]
    List,

    #[command(description = "Cancel/delete an event")]
    Cancel,

    #[command(description = "Manage CalDAV device passwords")]
    Device,

    #[command(description = "Export calendar as .ics file")]
    Export,

    #[command(description = "Show help message")]
    Help,

    #[command(description = "Delete your account and all data (GDPR)")]
    DeleteAccount,
}
