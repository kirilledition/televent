//! Command handlers
//!
//! Implementation of all bot command handlers

use crate::db::BotDb;
use crate::event_parser::{format_example, parse_event_message};
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
         /list - View upcoming events\n\
         /device - Manage CalDAV device passwords\n\
         /help - Show all commands\n\n\
         Your account is ready! Get started by sending a message to create your first event.";

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
         /list - List upcoming events\n\
         /cancel - Cancel an event\n\n\
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
pub async fn handle_export(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg
        .from
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Fetch all events for the user
    let events = db.get_all_events_for_user(telegram_id).await?;

    if events.is_empty() {
        bot.send_message(
            msg.chat.id,
            "📅 You don't have any events to export yet.",
        )
        .await?;
        return Ok(());
    }

    // Generate ICS content
    let ics_content = generate_ics(&events);

    // Send as file
    let file = teloxide::types::InputFile::memory(ics_content.into_bytes())
        .file_name("calendar.ics");

    bot.send_document(msg.chat.id, file)
        .caption("📤 Here is your calendar export.")
        .await?;

    tracing::info!("User {} exported {} events", telegram_id, events.len());

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
pub async fn handle_list(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg
        .from
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Get upcoming events (next 7 days)
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
        bot.send_message(msg.chat.id, "📅 No upcoming events in the next 7 days.")
            .await?;
    } else {
        let mut response = format!("📅 <b>Upcoming Events (Next 7 Days)</b> ({})\n\n", events.len());

        for (idx, event) in events.iter().enumerate() {
            response.push_str(&format!(
                "{}. <b>{}</b>\n   📆 {}\n   🕐 {}\n",
                idx + 1,
                event.summary,
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
        "User {} queried list events: {} found",
        telegram_id,
        events.len()
    );

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

/// Handle the /invite command
pub async fn handle_invite(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    use televent_core::attendee::generate_internal_email;
    use uuid::Uuid;

    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Parse command arguments: /invite <event_id> <@username or email>
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();

    if parts.len() < 3 {
        let help_text = "📨 <b>Invite Someone to an Event</b>\n\n\
                        <b>Usage:</b>\n\
                        /invite &lt;event_id&gt; @username\n\
                        /invite &lt;event_id&gt; email@example.com\n\n\
                        <b>Example:</b>\n\
                        /invite abc123... @alice\n\
                        /invite abc123... user@gmail.com";

        bot.send_message(msg.chat.id, help_text)
            .parse_mode(ParseMode::Html)
            .await?;
        return Ok(());
    }

    let event_id_str = parts[1];
    let invitee_str = parts[2];

    // Parse event UUID
    let event_id = match Uuid::parse_str(event_id_str) {
        Ok(id) => id,
        Err(_) => {
            bot.send_message(msg.chat.id, "❌ Invalid event ID format")
                .await?;
            return Ok(());
        }
    };

    // Verify user owns this event
    let event_info = match db.get_event_info(event_id, telegram_id).await? {
        Some(info) => info,
        None => {
            bot.send_message(
                msg.chat.id,
                "❌ Event not found or you don't have permission to invite others",
            )
            .await?;
            return Ok(());
        }
    };

    // Determine if invitee is internal (@ username) or external (email)
    let (invitee_email, invitee_telegram_id) = if invitee_str.starts_with('@') {
        // Internal invite - lookup Telegram user
        let username = invitee_str.trim_start_matches('@');
        match db.find_user_by_username(username).await? {
            Some(user_info) => (
                generate_internal_email(user_info.telegram_id),
                Some(user_info.telegram_id),
            ),
            None => {
                bot.send_message(
                    msg.chat.id,
                    format!("❌ User @{} not found. They need to /start the bot first.", username),
                )
                .await?;
                return Ok(());
            }
        }
    } else {
        // External invite - use email as-is
        (invitee_str.to_string(), None)
    };

    // Create attendee record
    match db
        .invite_attendee(event_id, &invitee_email, invitee_telegram_id, "ATTENDEE")
        .await
    {
        Ok(_) => {
            // Queue calendar invite message
            db.queue_calendar_invite(
                &invitee_email,
                invitee_telegram_id,
                &event_info.summary,
                event_info.start,
                event_info.location.as_deref(),
            )
            .await?;

            let success_msg = if invitee_telegram_id.is_some() {
                format!(
                    "✅ Invited {} to event: <b>{}</b>\n\nThey will receive a Telegram notification.",
                    invitee_str, event_info.summary
                )
            } else {
                format!(
                    "✅ Invited {} to event: <b>{}</b>\n\n⚠️ External invites are logged but not sent in MVP mode.",
                    invitee_str, event_info.summary
                )
            };

            bot.send_message(msg.chat.id, success_msg)
                .parse_mode(ParseMode::Html)
                .await?;

            tracing::info!(
                "User {} invited {} to event {}",
                telegram_id,
                invitee_email,
                event_id
            );
        }
        Err(sqlx::Error::RowNotFound) => {
            bot.send_message(
                msg.chat.id,
                format!("⚠️ {} is already invited to this event", invitee_str),
            )
            .await?;
        }
        Err(e) => {
            tracing::error!("Failed to invite attendee: {}", e);
            bot.send_message(
                msg.chat.id,
                "❌ Failed to send invite. Please try again later.",
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle the /rsvp command
pub async fn handle_rsvp(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;
    let username = user.username.clone().unwrap_or_else(|| format!("User_{}", telegram_id));

    // Parse command arguments: /rsvp [<event_id> <status>]
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();

    // If no arguments, list pending invites
    if parts.len() < 2 {
        let pending = db.get_pending_invites(telegram_id).await?;

        if pending.is_empty() {
            bot.send_message(msg.chat.id, "📭 No pending invitations")
                .await?;
            return Ok(());
        }

        let mut response = format!("📨 <b>Pending Invitations</b> ({})\n\n", pending.len());
        for invite in pending.iter().take(10) {
            let organizer = invite
                .organizer_username
                .as_ref()
                .map(|u| format!("@{}", u))
                .unwrap_or_else(|| "Unknown".to_string());

            let location_text = invite
                .location
                .as_ref()
                .map(|loc| format!("\n📍 {}", loc))
                .unwrap_or_default();

            response.push_str(&format!(
                "🔹 <b>{}</b>\n   🕒 {}\n   👤 From: {}{}\n   <code>/rsvp {} accept</code>\n\n",
                invite.summary,
                invite.start.format("%a %b %d at %H:%M UTC"),
                organizer,
                location_text,
                invite.event_id
            ));
        }

        response.push_str("\n<b>To respond:</b>\n");
        response.push_str("/rsvp &lt;event_id&gt; accept\n");
        response.push_str("/rsvp &lt;event_id&gt; decline\n");
        response.push_str("/rsvp &lt;event_id&gt; tentative");

        bot.send_message(msg.chat.id, response)
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(());
    }

    // Parse RSVP response: /rsvp <event_id> <status>
    if parts.len() < 3 {
        bot.send_message(
            msg.chat.id,
            "❌ Usage: /rsvp &lt;event_id&gt; &lt;accept|decline|tentative&gt;",
        )
        .parse_mode(ParseMode::Html)
        .await?;
        return Ok(());
    }

    let event_id_str = parts[1];
    let status_str = parts[2].to_lowercase();

    // Parse event UUID
    let event_id = match uuid::Uuid::parse_str(event_id_str) {
        Ok(id) => id,
        Err(_) => {
            bot.send_message(msg.chat.id, "❌ Invalid event ID format")
                .await?;
            return Ok(());
        }
    };

    // Map user input to participation status
    let status = match status_str.as_str() {
        "accept" | "accepted" | "yes" => "ACCEPTED",
        "decline" | "declined" | "no" => "DECLINED",
        "tentative" | "maybe" => "TENTATIVE",
        _ => {
            bot.send_message(
                msg.chat.id,
                "❌ Invalid status. Use: accept, decline, or tentative",
            )
            .await?;
            return Ok(());
        }
    };

    // Update RSVP status
    match db.update_rsvp_status(event_id, telegram_id, status).await {
        Ok(true) => {
            // Get organizer to notify
            if let Some(organizer_id) = db.get_event_organizer(event_id).await? {
                // Get event info for notification
                if let Some(event_info) = db.get_event_info(event_id, organizer_id).await? {
                    // Queue notification to organizer
                    db.queue_rsvp_notification(
                        organizer_id,
                        &username,
                        &event_info.summary,
                        status,
                    )
                    .await?;
                }
            }

            let emoji = match status {
                "ACCEPTED" => "✅",
                "DECLINED" => "❌",
                "TENTATIVE" => "🤔",
                _ => "✉️",
            };

            bot.send_message(
                msg.chat.id,
                format!("{} Your response has been recorded: <b>{}</b>", emoji, status),
            )
            .parse_mode(ParseMode::Html)
            .await?;

            tracing::info!(
                "User {} responded {} to event {}",
                telegram_id,
                status,
                event_id
            );
        }
        Ok(false) => {
            bot.send_message(msg.chat.id, "❌ Invitation not found")
                .await?;
        }
        Err(e) => {
            tracing::error!("Failed to update RSVP: {}", e);
            bot.send_message(
                msg.chat.id,
                "❌ Failed to update your response. Please try again later.",
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle non-command text messages (event creation)
///
/// This handler processes multi-line text messages as potential event creation requests.
/// Messages must have at least 2 lines (title and date/time) to be considered.
pub async fn handle_text_message(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()), // Not a text message
    };

    // Skip commands
    if text.starts_with('/') {
        return Ok(());
    }

    // Skip messages with fewer than 2 lines (not enough for event creation)
    let line_count = text.lines().count();
    if line_count < 2 {
        return Ok(());
    }

    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Try to parse as event
    match parse_event_message(text) {
        Ok(parsed_event) => {
            // Generate unique UID for the event
            let uid = format!("{}@televent.bot", uuid::Uuid::new_v4());
            let end_time = parsed_event.end_time();

            // Create event in database
            match db
                .create_event(
                    telegram_id,
                    &uid,
                    &parsed_event.title,
                    None, // description
                    parsed_event.location.as_deref(),
                    parsed_event.start,
                    end_time,
                    "UTC",
                )
                .await
            {
                Ok(event) => {
                    // Format confirmation message
                    let location_text = parsed_event
                        .location
                        .as_ref()
                        .map(|loc| format!("\n📍 <b>Location:</b> {}", loc))
                        .unwrap_or_default();

                    let response = format!(
                        "✅ <b>Event Created!</b>\n\n\
                         📌 <b>{}</b>\n\
                         📅 {}\n\
                         🕐 {} - {} ({} min){}\n\n\
                         Use /list to view your upcoming events.",
                        event.summary,
                        event.start.format("%A, %B %d, %Y"),
                        event.start.format("%H:%M"),
                        end_time.format("%H:%M"),
                        parsed_event.duration_minutes,
                        location_text
                    );

                    bot.send_message(msg.chat.id, response)
                        .parse_mode(ParseMode::Html)
                        .await?;

                    tracing::info!(
                        "User {} created event: {} at {}",
                        telegram_id,
                        event.summary,
                        event.start
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to create event for user {}: {}", telegram_id, e);

                    bot.send_message(
                        msg.chat.id,
                        "❌ Failed to create event. Please try again later.",
                    )
                    .await?;
                }
            }
        }
        Err(parse_error) => {
            // Send helpful error message
            let response = format!(
                "❌ <b>Could not create event</b>\n\n{}\n\n{}",
                parse_error,
                format_example()
            );

            bot.send_message(msg.chat.id, response)
                .parse_mode(ParseMode::Html)
                .await?;

            tracing::info!(
                "User {} sent invalid event format: {}",
                telegram_id,
                parse_error
            );
        }
    }

    Ok(())
}

/// Generate ICS content from a list of events
fn generate_ics(events: &[crate::db::BotEvent]) -> String {
    use icalendar::{Calendar, Component, Event, EventLike};

    let mut calendar = Calendar::new();
    calendar
        .name("Televent Calendar")
        .description("Exported from Televent Telegram Bot");

    for event in events {
        let mut ics_event = Event::new();
        ics_event
            .summary(&event.summary)
            .starts(event.start)
            .ends(event.end)
            .uid(&event.id.to_string());

        if let Some(desc) = &event.description {
            ics_event.description(desc);
        }
        if let Some(loc) = &event.location {
            ics_event.location(loc);
        }

        calendar.push(ics_event);
    }

    calendar.to_string()
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
        assert!(cmds_str.contains("list"), "Should contain /list command");
    }

    #[test]
    fn test_generate_ics() {
        use crate::db::BotEvent;
        use chrono::{TimeZone, Utc};
        use uuid::Uuid;

        let start = Utc.with_ymd_and_hms(2023, 10, 27, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2023, 10, 27, 11, 0, 0).unwrap();

        let event = BotEvent {
            id: Uuid::new_v4(),
            summary: "Test Event".to_string(),
            start,
            end,
            location: Some("Online".to_string()),
            description: Some("Description".to_string()),
        };

        let ics = super::generate_ics(&[event]);

        assert!(ics.contains("BEGIN:VCALENDAR"));
        assert!(ics.contains("BEGIN:VEVENT"));
        assert!(ics.contains("SUMMARY:Test Event"));
        assert!(ics.contains("LOCATION:Online"));
        assert!(ics.contains("DESCRIPTION:Description"));
        assert!(ics.contains("END:VEVENT"));
        assert!(ics.contains("END:VCALENDAR"));
    }
}
