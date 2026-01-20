//! Command handlers
//!
//! Implementation of all bot command handlers

use crate::db::BotDb;
use anyhow::Result;
use chrono::{Duration, Utc};
use teloxide::prelude::*;
use teloxide::types::ParseMode;

/// Handle the /start command
pub async fn handle_start(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;
    let username = user.username.as_deref();

    // Ensure user and calendar are set up
    if let Err(e) = db.ensure_user_setup(telegram_id, username).await {
        tracing::error!("Failed to setup user {}: {}", telegram_id, e);
        bot.send_message(
            msg.chat.id,
            "❌ Failed to initialize your account. Please try again later.",
        )
        .await?;
        return Ok(());
    }

    let welcome_text = "👋 <b>Welcome to Televent!</b>\n\n\
         Your Telegram-native calendar with CalDAV sync.\n\n\
         <b>Quick Commands:</b>\n\
         /today - View today's events\n\
         /tomorrow - View tomorrow's events\n\
         /week - View this week's events\n\
         /create - Create a new event\n\
         /device - Manage CalDAV device passwords\n\
         /help - Show all commands\n\n\
         Your account is ready! Get started by creating your first event with /create";

    bot.send_message(msg.chat.id, welcome_text)
        .parse_mode(ParseMode::Html)
        .await?;

    tracing::info!("User {} started the bot", telegram_id);

    Ok(())
}

/// Handle the /help command
pub async fn handle_help(bot: Bot, msg: Message) -> Result<()> {
    let help_text = "<b>Televent Commands</b>\n\n\
         <b>Event Management:</b>\n\
         /today - Show today's events\n\
         /tomorrow - Show tomorrow's events\n\
         /week - Show this week's events\n\
         /create - Create a new event\n\
         /cancel - Cancel an event\n\
         /list - List events in a date range\n\n\
         <b>CalDAV Sync:</b>\n\
         /device - Manage device passwords for CalDAV clients\n\
         /export - Export calendar as .ics file\n\n\
         <b>Account:</b>\n\
         /deleteaccount - Delete your account and all data\n\n\
         For detailed help, visit: https://github.com/kirilledition/televent";

    bot.send_message(msg.chat.id, help_text)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Handle the /today command
pub async fn handle_today(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg
        .from
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Get today's date range
    let now = Utc::now();
    let start_of_day = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
        .and_utc();
    let end_of_day = start_of_day + Duration::days(1);

    // Query events from database
    let events = db
        .get_events_for_user(telegram_id, start_of_day, end_of_day)
        .await?;

    if events.is_empty() {
        bot.send_message(msg.chat.id, "📅 No events today. Enjoy your free time!")
            .await?;
    } else {
        let mut response = format!("📅 <b>Today's Events</b> ({})\n\n", events.len());

        for (idx, event) in events.iter().enumerate() {
            response.push_str(&format!(
                "{}. <b>{}</b>\n   🕐 {}\n",
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
            .parse_mode(ParseMode::Html)
            .await?;
    }

    tracing::info!(
        "User {} queried today's events: {} found",
        telegram_id,
        events.len()
    );

    Ok(())
}

/// Handle the /tomorrow command
pub async fn handle_tomorrow(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg
        .from
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Get tomorrow's date range
    let tomorrow = Utc::now() + Duration::days(1);
    let start_of_day = tomorrow
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
        .and_utc();
    let end_of_day = start_of_day + Duration::days(1);

    // Query events from database
    let events = db
        .get_events_for_user(telegram_id, start_of_day, end_of_day)
        .await?;

    if events.is_empty() {
        bot.send_message(msg.chat.id, "📅 No events tomorrow.")
            .await?;
    } else {
        let mut response = format!("📅 <b>Tomorrow's Events</b> ({})\n\n", events.len());

        for (idx, event) in events.iter().enumerate() {
            response.push_str(&format!(
                "{}. <b>{}</b>\n   🕐 {}\n",
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
            .parse_mode(ParseMode::Html)
            .await?;
    }

    tracing::info!(
        "User {} queried tomorrow's events: {} found",
        telegram_id,
        events.len()
    );

    Ok(())
}

/// Handle the /week command
pub async fn handle_week(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg
        .from
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Get this week's date range (next 7 days)
    let now = Utc::now();
    let start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
        .and_utc();
    let end = start + Duration::days(7);

    // Query events from database
    let events = db.get_events_for_user(telegram_id, start, end).await?;

    if events.is_empty() {
        bot.send_message(msg.chat.id, "📅 No events this week.")
            .await?;
    } else {
        let mut response = format!("📅 <b>This Week's Events</b> ({})\n\n", events.len());

        for (idx, event) in events.iter().enumerate() {
            response.push_str(&format!(
                "{}. <b>{}</b>\n   📆 {}\n   🕐 {}\n",
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
            .parse_mode(ParseMode::Html)
            .await?;
    }

    tracing::info!(
        "User {} queried week's events: {} found",
        telegram_id,
        events.len()
    );

    Ok(())
}

/// Handle the /create command
pub async fn handle_create(bot: Bot, msg: Message) -> Result<()> {
    let response = "🎯 <b>Create New Event</b>\n\n\
                    To create an event, send a message in this format:\n\n\
                    <code>Event Title</code>\n\
                    <code>YYYY-MM-DD HH:MM</code>\n\
                    <code>Duration in minutes (optional)</code>\n\
                    <code>Location (optional)</code>\n\n\
                    <b>Example:</b>\n\
                    Team Meeting\n\
                    2026-01-20 14:00\n\
                    60\n\
                    Conference Room A\n\n\
                    Or use the web UI for a better experience!";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Handle the /device command
pub async fn handle_device(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg
        .from
        .clone()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Get text after command (if any)
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();

    match parts.get(1).copied() {
        Some("add") => {
            let device_name = parts
                .get(2..)
                .map(|s| s.join(" "))
                .unwrap_or_else(|| "My Device".to_string());

            match db.generate_device_password(telegram_id, &device_name).await {
                Ok(password) => {
                    let response = format!(
                        "✅ <b>Device Password Created!</b>\n\n\
                         🏷️ Device: {}\n\
                         🔑 Password: <code>{}</code>\n\n\
                         <b>CalDAV Setup:</b>\n\
                         Server: <code>https://your-domain.com/caldav</code>\n\
                         Username: <code>{}</code>\n\
                         Password: Use the password above\n\n\
                         ⚠️ <b>Important:</b> Save this password securely! \
                         You won't be able to see it again.",
                        device_name, password, telegram_id
                    );

                    bot.send_message(msg.chat.id, response)
                        .parse_mode(ParseMode::Html)
                        .await?;

                    tracing::info!(
                        "Device password created for user {}: {}",
                        telegram_id,
                        device_name
                    );
                }
                Err(e) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("❌ Failed to create device password: {}", e),
                    )
                    .await?;
                }
            }
        }
        Some("list") => match db.list_device_passwords(telegram_id).await {
            Ok(devices) if devices.is_empty() => {
                bot.send_message(
                    msg.chat.id,
                    "📱 You don't have any device passwords yet.\n\n\
                         Create one with: <code>/device add Device Name</code>",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            }
            Ok(devices) => {
                let mut response = format!("📱 <b>Your Devices</b> ({})\n\n", devices.len());

                for (idx, device) in devices.iter().enumerate() {
                    response.push_str(&format!(
                        "{}. <b>{}</b>\n   🆔 <code>{}</code>\n   📅 Created: {}\n",
                        idx + 1,
                        device.name,
                        device.id,
                        device.created_at.format("%Y-%m-%d %H:%M")
                    ));

                    if let Some(last_used) = device.last_used_at {
                        response.push_str(&format!(
                            "   🕐 Last used: {}\n",
                            last_used.format("%Y-%m-%d %H:%M")
                        ));
                    }

                    response.push('\n');
                }

                response.push_str("To revoke a device: <code>/device revoke &lt;ID&gt;</code>");

                bot.send_message(msg.chat.id, response)
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            Err(e) => {
                bot.send_message(msg.chat.id, format!("❌ Failed to list devices: {}", e))
                    .await?;
            }
        },
        Some("revoke") => {
            if let Some(device_id_str) = parts.get(2) {
                match device_id_str.parse::<uuid::Uuid>() {
                    Ok(device_id) => {
                        match db.revoke_device_password(telegram_id, device_id).await {
                            Ok(true) => {
                                bot.send_message(
                                    msg.chat.id,
                                    "✅ Device password revoked successfully!",
                                )
                                .await?;

                                tracing::info!(
                                    "Device password revoked for user {}: {}",
                                    telegram_id,
                                    device_id
                                );
                            }
                            Ok(false) => {
                                bot.send_message(
                                    msg.chat.id,
                                    "❌ Device not found or already revoked.",
                                )
                                .await?;
                            }
                            Err(e) => {
                                bot.send_message(
                                    msg.chat.id,
                                    format!("❌ Failed to revoke device: {}", e),
                                )
                                .await?;
                            }
                        }
                    }
                    Err(_) => {
                        bot.send_message(msg.chat.id, "❌ Invalid device ID format.")
                            .await?;
                    }
                }
            } else {
                bot.send_message(
                    msg.chat.id,
                    "❌ Please provide a device ID: <code>/device revoke &lt;ID&gt;</code>",
                )
                .parse_mode(ParseMode::Html)
                .await?;
            }
        }
        _ => {
            let response = "🔐 <b>CalDAV Device Management</b>\n\n\
                            Device passwords allow you to sync your calendar with CalDAV clients.\n\n\
                            <b>Commands:</b>\n\
                            <code>/device add [name]</code> - Create a new device password\n\
                            <code>/device list</code> - List all your devices\n\
                            <code>/device revoke &lt;id&gt;</code> - Revoke a device password\n\n\
                            <b>Supported Clients:</b>\n\
                            • Apple Calendar (iOS, macOS)\n\
                            • Thunderbird\n\
                            • DAVx⁵ (Android)\n\
                            • Any CalDAV-compatible client";

            bot.send_message(msg.chat.id, response)
                .parse_mode(ParseMode::Html)
                .await?;
        }
    }

    Ok(())
}

/// Handle the /export command
pub async fn handle_export(bot: Bot, msg: Message, _db: BotDb) -> Result<()> {
    let user = msg
        .from
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // TODO: Implement ICS export
    bot.send_message(
        msg.chat.id,
        "📤 Export functionality coming soon! For now, use CalDAV sync to access your calendar.",
    )
    .await?;

    tracing::info!("User {} requested calendar export", telegram_id);

    Ok(())
}

/// Handle the /deleteaccount command
pub async fn handle_delete_account(bot: Bot, msg: Message) -> Result<()> {
    let response = "⚠️ <b>Delete Account</b>\n\n\
                    This will permanently delete:\n\
                    • All your events\n\
                    • Your calendar\n\
                    • All device passwords\n\
                    • All associated data\n\n\
                    This action CANNOT be undone.\n\n\
                    To confirm deletion, use the web UI and follow the GDPR deletion process.";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Handle the /list command
pub async fn handle_list(bot: Bot, msg: Message) -> Result<()> {
    let response = "📋 <b>List Events</b>\n\n\
                    Use these commands for quick lists:\n\
                    /today - Today's events\n\
                    /tomorrow - Tomorrow's events\n\
                    /week - Next 7 days\n\n\
                    For custom date ranges, use the web UI.";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Handle the /cancel command
pub async fn handle_cancel(bot: Bot, msg: Message) -> Result<()> {
    let response = "❌ <b>Cancel Event</b>\n\n\
                    To cancel an event, use the web UI where you can:\n\
                    • View all your events\n\
                    • Select events to cancel\n\
                    • See event details before deletion";

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::commands::Command;
    use teloxide::utils::command::BotCommands;

    #[test]
    fn test_command_descriptions() {
        // Verify commands can be parsed
        let cmds = Command::descriptions();
        let cmds_str = cmds.to_string();
        assert!(cmds_str.contains("start"), "Should contain /start command");
        assert!(cmds_str.contains("today"), "Should contain /today command");
    }
}
