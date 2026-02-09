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

    #[command(description = "List upcoming events")]
    List,

    #[command(description = "Cancel/delete an event")]
    Cancel,

    #[command(description = "Manage CalDAV device passwords")]
    Device,

    #[command(description = "Export calendar as .ics file")]
    Export,

    #[command(description = "Invite someone to an event")]
    Invite,

    #[command(description = "Respond to event invitations")]
    Rsvp,

    #[command(description = "Show help message")]
    Help,

    #[command(description = "Delete your account and all data (GDPR)")]
    DeleteAccount,
}
