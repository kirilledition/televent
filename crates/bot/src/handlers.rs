//! Command handlers
//!
//! Implementation of all bot command handlers

use crate::commands::Command;
use crate::db::BotDb;
use anyhow::Result;
use chrono::{Duration, Utc};
use teloxide::prelude::*;
use teloxide::types::ParseMode;

/// Handle the /start command
pub async fn handle_start(bot: Bot, msg: Message, _db: BotDb) -> Result<()> {
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64);

    let welcome_text = format!(
        "👋 *Welcome to Televent!*\n\n\
         Your Telegram-native calendar with CalDAV sync.\n\n\
         *Quick Commands:*\n\
         /today - View today's events\n\
         /tomorrow - View tomorrow's events\n\
         /week - View this week's events\n\
         /create - Create a new event\n\
         /device - Manage CalDAV device passwords\n\
         /help - Show all commands\n\n\
         Get started by creating your first event with /create!"
    );

    bot.send_message(msg.chat.id, welcome_text)
        .parse_mode(ParseMode::Markdown)
        .await?;

    tracing::info!("User {:?} started the bot", user_id);

    Ok(())
}

/// Handle the /help command
pub async fn handle_help(bot: Bot, msg: Message) -> Result<()> {
    let help_text = format!(
        "*Televent Commands*\n\n\
         *Event Management:*\n\
         /today - Show today's events\n\
         /tomorrow - Show tomorrow's events\n\
         /week - Show this week's events\n\
         /create - Create a new event\n\
         /cancel - Cancel an event\n\
         /list - List events in a date range\n\n\
         *CalDAV Sync:*\n\
         /device - Manage device passwords for CalDAV clients\n\
         /export - Export calendar as .ics file\n\n\
         *Account:*\n\
         /deleteaccount - Delete your account and all data\n\n\
         For detailed help, visit: https://github.com/kirilledition/televent"
    );

    bot.send_message(msg.chat.id, help_text)
        .parse_mode(ParseMode::Markdown)
        .await?;

    Ok(())
}

/// Handle the /today command
pub async fn handle_today(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg.from.ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Get today's date range
    let now = Utc::now();
    let start_of_day = now.date_naive().and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
        .and_utc();
    let end_of_day = start_of_day + Duration::days(1);

    // Query events from database
    let events = db.get_events_for_user(telegram_id, start_of_day, end_of_day).await?;

    if events.is_empty() {
        bot.send_message(msg.chat.id, "📅 No events today. Enjoy your free time!")
            .await?;
    } else {
        let mut response = format!("📅 *Today's Events* ({})\n\n", events.len());

        for (idx, event) in events.iter().enumerate() {
            response.push_str(&format!(
                "{}. *{}*\n   🕐 {}\n",
                idx + 1,
                event.title,
                event.start.format("%H:%M")
            ));

            if let Some(location) = &event.location {
                response.push_str(&format!("   📍 {}\n", location));
            }

            response.push('\n');
        }

        bot.send_message(msg.chat.id, response)
            .parse_mode(ParseMode::Markdown)
            .await?;
    }

    tracing::info!("User {} queried today's events: {} found", telegram_id, events.len());

    Ok(())
}

/// Handle the /tomorrow command
pub async fn handle_tomorrow(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg.from.ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Get tomorrow's date range
    let tomorrow = Utc::now() + Duration::days(1);
    let start_of_day = tomorrow.date_naive().and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
        .and_utc();
    let end_of_day = start_of_day + Duration::days(1);

    // Query events from database
    let events = db.get_events_for_user(telegram_id, start_of_day, end_of_day).await?;

    if events.is_empty() {
        bot.send_message(msg.chat.id, "📅 No events tomorrow.")
            .await?;
    } else {
        let mut response = format!("📅 *Tomorrow's Events* ({})\n\n", events.len());

        for (idx, event) in events.iter().enumerate() {
            response.push_str(&format!(
                "{}. *{}*\n   🕐 {}\n",
                idx + 1,
                event.title,
                event.start.format("%H:%M")
            ));

            if let Some(location) = &event.location {
                response.push_str(&format!("   📍 {}\n", location));
            }

            response.push('\n');
        }

        bot.send_message(msg.chat.id, response)
            .parse_mode(ParseMode::Markdown)
            .await?;
    }

    tracing::info!("User {} queried tomorrow's events: {} found", telegram_id, events.len());

    Ok(())
}

/// Handle the /week command
pub async fn handle_week(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg.from.ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Get this week's date range (next 7 days)
    let now = Utc::now();
    let start = now.date_naive().and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
        .and_utc();
    let end = start + Duration::days(7);

    // Query events from database
    let events = db.get_events_for_user(telegram_id, start, end).await?;

    if events.is_empty() {
        bot.send_message(msg.chat.id, "📅 No events this week.")
            .await?;
    } else {
        let mut response = format!("📅 *This Week's Events* ({})\n\n", events.len());

        for (idx, event) in events.iter().enumerate() {
            response.push_str(&format!(
                "{}. *{}*\n   📆 {}\n   🕐 {}\n",
                idx + 1,
                event.title,
                event.start.format("%a, %b %d"),
                event.start.format("%H:%M")
            ));

            if let Some(location) = &event.location {
                response.push_str(&format!("   📍 {}\n", location));
            }

            response.push('\n');
        }

        bot.send_message(msg.chat.id, response)
            .parse_mode(ParseMode::Markdown)
            .await?;
    }

    tracing::info!("User {} queried week's events: {} found", telegram_id, events.len());

    Ok(())
}

/// Handle the /create command
pub async fn handle_create(bot: Bot, msg: Message) -> Result<()> {
    let response = "🎯 *Create New Event*\n\n\
                    To create an event, send a message in this format:\n\n\
                    `Event Title`\n\
                    `YYYY-MM-DD HH:MM`\n\
                    `Duration in minutes (optional)`\n\
                    `Location (optional)`\n\n\
                    *Example:*\n\
                    Team Meeting\n\
                    2026-01-20 14:00\n\
                    60\n\
                    Conference Room A\n\n\
                    Or use the web UI for a better experience!";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Markdown)
        .await?;

    Ok(())
}

/// Handle the /device command
pub async fn handle_device(bot: Bot, msg: Message) -> Result<()> {
    let response = "🔐 *CalDAV Device Management*\n\n\
                    Device passwords allow you to sync your calendar with CalDAV clients \
                    (Apple Calendar, Google Calendar, Thunderbird, etc.)\n\n\
                    Use the web UI to:\n\
                    • Create device passwords\n\
                    • List existing devices\n\
                    • Delete device access\n\n\
                    CalDAV URL: `https://your-domain.com/caldav`\n\
                    Username: Your Telegram ID\n\
                    Password: Generated device password";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Markdown)
        .await?;

    Ok(())
}

/// Handle the /export command
pub async fn handle_export(bot: Bot, msg: Message, _db: BotDb) -> Result<()> {
    let user = msg.from.ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // TODO: Implement ICS export
    bot.send_message(
        msg.chat.id,
        "📤 Export functionality coming soon! For now, use CalDAV sync to access your calendar."
    )
    .await?;

    tracing::info!("User {} requested calendar export", telegram_id);

    Ok(())
}

/// Handle the /deleteaccount command
pub async fn handle_delete_account(bot: Bot, msg: Message) -> Result<()> {
    let response = "⚠️ *Delete Account*\n\n\
                    This will permanently delete:\n\
                    • All your events\n\
                    • Your calendar\n\
                    • All device passwords\n\
                    • All associated data\n\n\
                    This action CANNOT be undone.\n\n\
                    To confirm deletion, use the web UI and follow the GDPR deletion process.";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Markdown)
        .await?;

    Ok(())
}

/// Handle the /list command
pub async fn handle_list(bot: Bot, msg: Message) -> Result<()> {
    let response = "📋 *List Events*\n\n\
                    Use these commands for quick lists:\n\
                    /today - Today's events\n\
                    /tomorrow - Tomorrow's events\n\
                    /week - Next 7 days\n\n\
                    For custom date ranges, use the web UI.";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Markdown)
        .await?;

    Ok(())
}

/// Handle the /cancel command
pub async fn handle_cancel(bot: Bot, msg: Message) -> Result<()> {
    let response = "❌ *Cancel Event*\n\n\
                    To cancel an event, use the web UI where you can:\n\
                    • View all your events\n\
                    • Select events to cancel\n\
                    • See event details before deletion";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Markdown)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_descriptions() {
        // Verify commands can be parsed
        let cmds = Command::descriptions();
        assert!(cmds.to_string().contains("Start"));
        assert!(cmds.to_string().contains("Today"));
    }
}
