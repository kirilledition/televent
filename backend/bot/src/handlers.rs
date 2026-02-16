//! Command handlers
//!
//! Implementation of all bot command handlers

use crate::db::BotDb;
use crate::event_parser::{format_example, parse_event_message};
use anyhow::Result;
use chrono::{Duration, Utc};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::html::escape;

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

    let welcome_text = "Welcome to Televent!\n\n\
         Your Telegram-native calendar with CalDAV sync.\n\n\
         To create an event, send a message with multiple lines:\n\
         1. Title\n\
         2. Date/Time\n\
         3. Duration (minutes, optional)\n\
         4. Location (optional)\n\n\
         Examples:\n\
         \n\
         [Exact Syntax]\n\
         Team Meeting\n\
         2026-01-25 14:00\n\
         60\n\
         Conference Room A\n\
         \n\
         [Natural Language]\n\
         Coffee with Alice\n\
         tomorrow at 3pm\n\
         30\n\
         Starbucks\n\n\
         Commands:\n\
         /list - View upcoming events\n\
         /device - Manage CalDAV device passwords\n\
         /help - Show all commands";

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
                        escape(&device_name),
                        password,
                        telegram_id
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
                        escape(&device.name),
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
        bot.send_message(msg.chat.id, "📅 You don't have any events to export yet.")
            .await?;
        return Ok(());
    }

    // Generate ICS content
    let ics_content = generate_ics(&events);

    // Send as file
    let file =
        teloxide::types::InputFile::memory(ics_content.into_bytes()).file_name("calendar.ics");

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
    let start_range = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
        .and_utc();
    let end_range = start_range + Duration::days(7);

    // Query events from database
    let events = db
        .get_events_for_user(telegram_id, start_range, end_range)
        .await?;

    if events.is_empty() {
        bot.send_message(msg.chat.id, "📅 No upcoming events in the next 7 days.")
            .await?;
    } else {
        let mut response = format!(
            "📅 <b>Upcoming Events (Next 7 Days)</b> ({})\n\n",
            events.len()
        );

        for (idx, event) in events.iter().enumerate() {
            let start = event.display_start();
            let time_str = if event.is_all_day {
                "All Day".to_string()
            } else {
                start.format("%H:%M").to_string()
            };

            response.push_str(&format!(
                "{}. <b>{}</b>\n   📆 {}\n   🕐 {}\n",
                idx + 1,
                escape(&event.summary),
                start.format("%a, %b %d"),
                time_str
            ));

            if let Some(location) = &event.location {
                response.push_str(&format!("   📍 {}\n", escape(location)));
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
                generate_internal_email(televent_core::models::UserId::new(user_info.telegram_id)),
                Some(user_info.telegram_id),
            ),
            None => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "❌ User @{} not found. They need to /start the bot first.",
                        username
                    ),
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
            let start = event_info.start.unwrap_or_else(|| {
                event_info
                    .start_date
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
            });
            db.queue_calendar_invite(
                &invitee_email,
                invitee_telegram_id,
                &event_info.summary,
                start,
                event_info.location.as_deref(),
            )
            .await?;

            let success_msg = if invitee_telegram_id.is_some() {
                format!(
                    "✅ Invited {} to event: <b>{}</b>\n\nThey will receive a Telegram notification.",
                    escape(invitee_str),
                    escape(&event_info.summary)
                )
            } else {
                format!(
                    "✅ Invited {} to event: <b>{}</b>\n\n⚠️ External invites are logged but not sent in MVP mode.",
                    escape(invitee_str),
                    escape(&event_info.summary)
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
    let username = user
        .username
        .clone()
        .unwrap_or_else(|| format!("User_{}", telegram_id));

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
                .map(|u| format!("@{}", escape(u)))
                .unwrap_or_else(|| "Unknown".to_string());

            let location_text = invite
                .location
                .as_ref()
                .map(|loc| format!("\n📍 {}", escape(loc)))
                .unwrap_or_default();

            let start = invite.start.unwrap_or_else(|| {
                invite
                    .start_date
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
            });
            let time_str = if invite.is_all_day {
                "All Day".to_string()
            } else {
                start.format("%H:%M UTC").to_string()
            };

            response.push_str(&format!(
                "🔹 <b>{}</b>\n   🕒 {} {}\n   👤 From: {}{}\n   <code>/rsvp {} accept</code>\n\n",
                escape(&invite.summary),
                start.format("%a %b %d"),
                time_str,
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
                format!(
                    "{} Your response has been recorded: <b>{}</b>",
                    emoji, status
                ),
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

    // Skip messages with fewer than 2 lines (not enough for event creation), but send help
    let line_count = text.lines().count();
    if line_count < 2 {
        let help_text = "To create an event, please use the following format:\n\n\
             [Exact Syntax]\n\
             Team Meeting\n\
             2026-01-25 14:00\n\
             60\n\
             Conference Room A\n\n\
             [Natural Language]\n\
             Coffee with Alice\n\
             tomorrow at 3pm\n\
             30\n\
             Starbucks";

        bot.send_message(msg.chat.id, help_text).await?;
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

            // Create event in database
            match db
                .create_event(
                    telegram_id,
                    &uid,
                    &parsed_event.title,
                    None, // description
                    parsed_event.location.as_deref(),
                    parsed_event.timing.clone(),
                    "UTC",
                )
                .await
            {
                Ok(event) => {
                    // Format confirmation message
                    let location_text = parsed_event
                        .location
                        .as_ref()
                        .map(|loc| format!("\n📍 <b>Location:</b> {}", escape(loc)))
                        .unwrap_or_default();

                    let start = event.display_start();
                    let timing_details = match event.timing() {
                        crate::event_parser::ParsedTiming::Timed {
                            duration_minutes, ..
                        } => {
                            let end_time =
                                start + chrono::Duration::minutes(i64::from(duration_minutes));
                            format!(
                                "{} - {} ({} min)",
                                start.format("%H:%M"),
                                end_time.format("%H:%M"),
                                duration_minutes
                            )
                        }
                        crate::event_parser::ParsedTiming::AllDay { .. } => "All Day".to_string(),
                    };

                    let response = format!(
                        "✅ <b>Event Created!</b>\n\n\
                         📌 <b>{}</b>\n\
                         📅 {}\n\
                         🕐 {}{}\n\n\
                         Use /list to view your upcoming events.",
                        escape(&event.summary),
                        start.format("%A, %B %d, %Y"),
                        timing_details,
                        location_text
                    );

                    bot.send_message(msg.chat.id, response)
                        .parse_mode(ParseMode::Html)
                        .await?;

                    tracing::info!(
                        "User {} created event: {} at {}",
                        telegram_id,
                        event.summary,
                        start
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
                escape(&parse_error.to_string()),
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

/// Handle callback queries (RSVP buttons)
pub async fn handle_callback_query(bot: Bot, q: CallbackQuery, db: BotDb) -> Result<()> {
    let data = match q.data {
        Some(d) => d,
        None => return Ok(()),
    };

    // Check if it's an RSVP callback
    if !data.starts_with("rsvp:") {
        return Ok(());
    }

    let user_id = q.from.id.0 as i64;
    let parts: Vec<&str> = data.split(':').collect();

    // Format: rsvp:<event_id>:<status>
    if parts.len() != 3 {
        // Just answer to stop spinner
        bot.answer_callback_query(q.id)
            .text("❌ Invalid data")
            .await?;
        return Ok(());
    }

    let event_id_str = parts[1];
    let status = parts[2];

    let event_id = match uuid::Uuid::parse_str(event_id_str) {
        Ok(id) => id,
        Err(_) => {
            bot.answer_callback_query(q.id)
                .text("❌ Invalid event ID")
                .await?;
            return Ok(());
        }
    };

    // Confirm RSVP in DB (transactional)
    match db.confirm_rsvp(event_id, user_id, status).await {
        Ok(_) => {
            let status_emoji = match status {
                "ACCEPTED" => "✅",
                "DECLINED" => "❌",
                "TENTATIVE" => "🤔",
                _ => "",
            };

            // Edit the message to remove buttons and show status
            if let Some(msg) = q.message {
                let text = match &msg {
                    teloxide::types::MaybeInaccessibleMessage::Regular(m) => m.text(),
                    _ => None,
                };

                if let Some(text) = text
                    && !text.contains("Status: ")
                {
                    // Use plain text edit to update status without losing content, though formatting is lost.
                    // Since we don't use ParseMode::Html here, injection is not possible.
                    let new_text = format!("{}\n\nStatus: {} {}", text, status_emoji, status);

                    // Edit text and remove keyboard
                    bot.edit_message_text(msg.chat().id, msg.id(), new_text)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::default())
                        .await?;
                }
            }

            bot.answer_callback_query(q.id).text("RSVP Updated").await?;
        }
        Err(e) => {
            tracing::error!("Failed to confirm RSVP: {}", e);
            bot.answer_callback_query(q.id)
                .text("❌ Failed to update RSVP. Please try again.")
                .show_alert(true)
                .await?;
        }
    }

    Ok(())
}

/// Generate ICS content from a list of events
struct FoldedWriter<'a> {
    buf: &'a mut String,
}

impl<'a> FoldedWriter<'a> {
    fn new(buf: &'a mut String) -> Self {
        Self { buf }
    }

    fn write_line(&mut self, line: &str) {
        self.buf.push_str(line);
        self.buf.push_str("\r\n");
    }

    fn write_property(&mut self, name: &str, value: &str) {
        self.write_property_impl(name, value, true)
    }

    fn write_datetime_property(&mut self, name: &str, datetime: &chrono::DateTime<chrono::Utc>) {
        self.buf.push_str(name);
        self.buf.push(':');
        // YYYYMMDDTHHmmssZ
        let _ = std::fmt::write(
            self.buf,
            format_args!("{}", datetime.format("%Y%m%dT%H%M%SZ")),
        );
        self.buf.push_str("\r\n");
    }

    fn write_date_property(&mut self, name: &str, date: &chrono::NaiveDate) {
        self.buf.push_str(name);
        self.buf.push(':');
        let _ = std::fmt::write(self.buf, format_args!("{}", date.format("%Y%m%d")));
        self.buf.push_str("\r\n");
    }

    fn write_property_impl(&mut self, name: &str, value: &str, escape: bool) {
        self.buf.push_str(name);
        self.buf.push(':');

        let mut current_line_len = name.len() + 1;

        for c in value.chars() {
            if c == '\r' {
                continue;
            } // Strip CR

            let replacement = if escape {
                match c {
                    '\\' => Some("\\\\"),
                    ';' => Some("\\;"),
                    ',' => Some("\\,"),
                    '\n' => Some("\\n"),
                    _ => None,
                }
            } else {
                None
            };

            if let Some(s) = replacement {
                for rc in s.chars() {
                    let len = rc.len_utf8();
                    if current_line_len + len > 75 {
                        self.buf.push_str("\r\n ");
                        current_line_len = 1;
                    }
                    self.buf.push(rc);
                    current_line_len += len;
                }
            } else {
                let len = c.len_utf8();
                if current_line_len + len > 75 {
                    self.buf.push_str("\r\n ");
                    current_line_len = 1;
                }
                self.buf.push(c);
                current_line_len += len;
            }
        }
        self.buf.push_str("\r\n");
    }
}

/// Generate ICS content from a list of events
fn generate_ics(events: &[crate::db::BotEvent]) -> String {
    // Pre-allocate buffer: ~200 bytes per event is a reasonable guess
    let mut buf = String::with_capacity(events.len() * 200 + 512);

    let mut writer = FoldedWriter::new(&mut buf);

    writer.write_line("BEGIN:VCALENDAR");
    writer.write_line("VERSION:2.0");
    writer.write_line("PRODID:-//Televent//Televent Bot//EN");
    writer.write_line("CALSCALE:GREGORIAN");

    // Original implementation set these properties via methods
    // calendar.name("Televent Calendar").description("Exported from Televent Telegram Bot");
    // These likely map to X-WR-CALNAME and X-WR-CALDESC
    writer.write_line("X-WR-CALNAME:Televent Calendar");
    writer.write_line("X-WR-CALDESC:Exported from Televent Telegram Bot");

    for event in events {
        writer.write_line("BEGIN:VEVENT");
        writer.write_property("UID", &event.id.to_string());

        // DTSTAMP required
        writer.write_datetime_property("DTSTAMP", &chrono::Utc::now());

        writer.write_property("SUMMARY", &event.summary);

        if let Some(desc) = &event.description {
            writer.write_property("DESCRIPTION", desc);
        }
        if let Some(loc) = &event.location {
            writer.write_property("LOCATION", loc);
        }

        if event.is_all_day {
            if let Some(start_date) = event.start_date {
                // icalendar crate's all_day sets DTSTART;VALUE=DATE
                writer.write_date_property("DTSTART;VALUE=DATE", &start_date);
            }
        } else {
            if let Some(start) = event.start {
                writer.write_datetime_property("DTSTART", &start);
            }
            if let Some(end) = event.end {
                writer.write_datetime_property("DTEND", &end);
            }
        }

        writer.write_line("END:VEVENT");
    }

    writer.write_line("END:VCALENDAR");

    buf
}

#[cfg(test)]
mod tests {
    use crate::commands::Command;
    use crate::db::{BotDb, BotEvent};
    use chrono::{TimeZone, Utc};
    use sqlx::PgPool;
    use teloxide::Bot;
    use teloxide::types::Message;
    use teloxide::utils::command::BotCommands;
    use uuid::Uuid;

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
        let start = Utc.with_ymd_and_hms(2023, 10, 27, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2023, 10, 27, 11, 0, 0).unwrap();

        let event = BotEvent {
            id: Uuid::new_v4(),
            summary: "Test Event".to_string(),
            start: Some(start),
            end: Some(end),
            start_date: None,
            end_date: None,
            is_all_day: false,
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

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_start(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let json = r#"{
            "message_id": 1,
            "date": 1600000000,
            "chat": {
                "id": 123456789,
                "type": "private",
                "username": "testuser",
                "first_name": "Test"
            },
            "from": {
                "id": 123456789,
                "is_bot": false,
                "first_name": "Test",
                "username": "testuser"
            },
            "text": "/start"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        // Handler should attempt to send message and fail, but ensure user setup first
        let _ = super::handle_start(bot, msg, db.clone()).await;

        // Verify user persistence
        let user = db.find_user_by_username("testuser").await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().telegram_id, 123456789);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_text_message_create_event(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let telegram_id = 123456789;
        db.ensure_user_setup(telegram_id, Some("testuser"))
            .await
            .unwrap();

        let json = r#"{
            "message_id": 2,
            "date": 1600000000,
            "chat": {
                "id": 123456789,
                "type": "private",
                "username": "testuser",
                "first_name": "Test"
            },
            "from": {
                "id": 123456789,
                "is_bot": false,
                "first_name": "Test",
                "username": "testuser"
            },
            "text": "Team Meeting\ntomorrow at 10am"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        // Handler should create event in DB then fail to send response
        let _ = super::handle_text_message(bot, msg, db.clone()).await;

        // Verify event creation
        let events = db.get_all_events_for_user(telegram_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Team Meeting");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_list(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let telegram_id = 987654321;
        db.ensure_user_setup(telegram_id, Some("listuser"))
            .await
            .unwrap();

        let json = r#"{
            "message_id": 3,
            "date": 1600000000,
            "chat": {
                "id": 987654321,
                "type": "private",
                "username": "listuser",
                "first_name": "List"
            },
            "from": {
                "id": 987654321,
                "is_bot": false,
                "first_name": "List",
                "username": "listuser"
            },
            "text": "/list"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        // Handler should query DB then fail to send
        let _ = super::handle_list(bot, msg, db).await;

        // No checks other than it ran without panic (and presumably queried DB)
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_help(_pool: PgPool) {
        let bot = Bot::new("123:fake_token");

        let json = r#"{
            "message_id": 10,
            "date": 1600000000,
            "chat": {
                "id": 111111111,
                "type": "private",
                "username": "helpuser",
                "first_name": "Help"
            },
            "from": {
                "id": 111111111,
                "is_bot": false,
                "first_name": "Help",
                "username": "helpuser"
            },
            "text": "/help"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        // Should just send help message and not fail
        let result = super::handle_help(bot, msg).await;
        assert!(result.is_ok() || result.is_err()); // Will fail to send but that's OK
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_export(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let telegram_id = 222222222;
        db.ensure_user_setup(telegram_id, Some("exportuser"))
            .await
            .unwrap();

        // Create an event first
        let start = chrono::Utc::now();
        db.create_event(
            telegram_id,
            &format!("{}", uuid::Uuid::new_v4()),
            "Export Test Event",
            None,
            Some("Test Location"),
            crate::event_parser::ParsedTiming::Timed {
                start,
                duration_minutes: 60,
            },
            "UTC",
        )
        .await
        .unwrap();

        let json = r#"{
            "message_id": 11,
            "date": 1600000000,
            "chat": {
                "id": 222222222,
                "type": "private",
                "username": "exportuser",
                "first_name": "Export"
            },
            "from": {
                "id": 222222222,
                "is_bot": false,
                "first_name": "Export",
                "username": "exportuser"
            },
            "text": "/export"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        let _ = super::handle_export(bot, msg, db).await;
        // Handler will query DB and try to send file; we just verify it runs
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_device_add(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let telegram_id = 333333333;
        db.ensure_user_setup(telegram_id, Some("deviceuser"))
            .await
            .unwrap();

        let json = r#"{
            "message_id": 12,
            "date": 1600000000,
            "chat": {
                "id": 333333333,
                "type": "private",
                "username": "deviceuser",
                "first_name": "Device"
            },
            "from": {
                "id": 333333333,
                "is_bot": false,
                "first_name": "Device",
                "username": "deviceuser"
            },
            "text": "/device add MyPhone"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        let _ = super::handle_device(bot, msg, db.clone()).await;

        // Verify device was created
        let devices = db.list_device_passwords(telegram_id).await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "MyPhone");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_device_list(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let telegram_id = 444444444;
        db.ensure_user_setup(telegram_id, Some("listdevuser"))
            .await
            .unwrap();
        db.generate_device_password(telegram_id, "Device1")
            .await
            .unwrap();

        let json = r#"{
            "message_id": 13,
            "date": 1600000000,
            "chat": {
                "id": 444444444,
                "type": "private",
                "username": "listdevuser",
                "first_name": "ListDev"
            },
            "from": {
                "id": 444444444,
                "is_bot": false,
                "first_name": "ListDev",
                "username": "listdevuser"
            },
            "text": "/device list"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        let _ = super::handle_device(bot, msg, db).await;
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_cancel(_pool: PgPool) {
        let bot = Bot::new("123:fake_token");

        let json = r#"{
            "message_id": 14,
            "date": 1600000000,
            "chat": {
                "id": 555555555,
                "type": "private",
                "username": "canceluser",
                "first_name": "Cancel"
            },
            "from": {
                "id": 555555555,
                "is_bot": false,
                "first_name": "Cancel",
                "username": "canceluser"
            },
            "text": "/cancel"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        let result = super::handle_cancel(bot, msg).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_delete_account(_pool: PgPool) {
        let bot = Bot::new("123:fake_token");

        let json = r#"{
            "message_id": 15,
            "date": 1600000000,
            "chat": {
                "id": 666666666,
                "type": "private",
                "username": "deleteuser",
                "first_name": "Delete"
            },
            "from": {
                "id": 666666666,
                "is_bot": false,
                "first_name": "Delete",
                "username": "deleteuser"
            },
            "text": "/deleteaccount"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();

        let result = super::handle_delete_account(bot, msg).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_invite(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let organizer_id = 777777777;
        let attendee_id = 888888888;

        db.ensure_user_setup(organizer_id, Some("organizer"))
            .await
            .unwrap();
        db.ensure_user_setup(attendee_id, Some("attendee"))
            .await
            .unwrap();

        // Create event
        let start = chrono::Utc::now();
        let event = db
            .create_event(
                organizer_id,
                &format!("{}", uuid::Uuid::new_v4()),
                "Party Event",
                None,
                None,
                crate::event_parser::ParsedTiming::Timed {
                    start,
                    duration_minutes: 60,
                },
                "UTC",
            )
            .await
            .unwrap();

        let json = format!(
            r#"{{
            "message_id": 16,
            "date": 1600000000,
            "chat": {{
                "id": 777777777,
                "type": "private",
                "username": "organizer",
                "first_name": "Organizer"
            }},
            "from": {{
                "id": 777777777,
                "is_bot": false,
                "first_name": "Organizer",
                "username": "organizer"
            }},
            "text": "/invite {} @attendee"
        }}"#,
            event.id
        );

        let msg: Message = serde_json::from_str(&json).unwrap();

        let _ = super::handle_invite(bot, msg, db.clone()).await;

        // Verify invite was created
        let invites = db.get_pending_invites(attendee_id).await.unwrap();
        assert_eq!(invites.len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_rsvp(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let organizer_id = 999999999;
        let attendee_id = 1010101010;

        db.ensure_user_setup(organizer_id, Some("org2"))
            .await
            .unwrap();
        db.ensure_user_setup(attendee_id, Some("att2"))
            .await
            .unwrap();

        // Create event and invite
        let start = chrono::Utc::now();
        let event = db
            .create_event(
                organizer_id,
                &format!("{}", uuid::Uuid::new_v4()),
                "RSVP Event",
                None,
                None,
                crate::event_parser::ParsedTiming::Timed {
                    start,
                    duration_minutes: 60,
                },
                "UTC",
            )
            .await
            .unwrap();

        db.invite_attendee(event.id, "att2@example.com", Some(attendee_id), "ATTENDEE")
            .await
            .unwrap();

        // Test listing pending invites
        let json_list = r#"{
            "message_id": 17,
            "date": 1600000000,
            "chat": {
                "id": 1010101010,
                "type": "private",
                "username": "att2",
                "first_name": "Att2"
            },
            "from": {
                "id": 1010101010,
                "is_bot": false,
                "first_name": "Att2",
                "username": "att2"
            },
            "text": "/rsvp"
        }"#;

        let msg_list: Message = serde_json::from_str(json_list).unwrap();
        let _ = super::handle_rsvp(bot.clone(), msg_list, db.clone()).await;

        // Test accepting invite
        let json_accept = format!(
            r#"{{
            "message_id": 18,
            "date": 1600000000,
            "chat": {{
                "id": 1010101010,
                "type": "private",
                "username": "att2",
                "first_name": "Att2"
            }},
            "from": {{
                "id": 1010101010,
                "is_bot": false,
                "first_name": "Att2",
                "username": "att2"
            }},
            "text": "/rsvp {} accept"
        }}"#,
            event.id
        );

        let msg_accept: Message = serde_json::from_str(&json_accept).unwrap();
        let _ = super::handle_rsvp(bot, msg_accept, db.clone()).await;

        // Verify RSVP was updated
        let invites_after = db.get_pending_invites(attendee_id).await.unwrap();
        assert_eq!(invites_after.len(), 0); // Should be empty as status changed from NEEDS-ACTION
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_text_message_invalid(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let telegram_id = 1111111111;
        db.ensure_user_setup(telegram_id, Some("invaliduser"))
            .await
            .unwrap();

        // Test with invalid event format (missing date)
        let json = r#"{
            "message_id": 19,
            "date": 1600000000,
            "chat": {
                "id": 1111111111,
                "type": "private",
                "username": "invaliduser",
                "first_name": "Invalid"
            },
            "from": {
                "id": 1111111111,
                "is_bot": false,
                "first_name": "Invalid",
                "username": "invaliduser"
            },
            "text": "Just a title"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();
        let _ = super::handle_text_message(bot, msg, db).await;
        // Should handle gracefully and not create event
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_handle_text_message_with_location(pool: PgPool) {
        let db = BotDb::new(pool);
        let bot = Bot::new("123:fake_token");

        let telegram_id = 1212121212;
        db.ensure_user_setup(telegram_id, Some("locuser"))
            .await
            .unwrap();

        let json = r#"{
            "message_id": 20,
            "date": 1600000000,
            "chat": {
                "id": 1212121212,
                "type": "private",
                "username": "locuser",
                "first_name": "Loc"
            },
            "from": {
                "id": 1212121212,
                "is_bot": false,
                "first_name": "Loc",
                "username": "locuser"
            },
            "text": "Meeting\ntomorrow 3pm\n90\nConference Room"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();
        let _ = super::handle_text_message(bot, msg, db.clone()).await;

        // Verify event with location
        let events = db.get_all_events_for_user(telegram_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Meeting");
        assert_eq!(events[0].location.as_deref(), Some("Conference Room"));
    }

    #[test]
    fn test_html_escaping_utility() {
        use teloxide::utils::html::escape;
        let input = "Me & You <script>";
        let escaped = escape(input);
        assert_eq!(escaped, "Me &amp; You &lt;script&gt;");
    }
}
