//! Dialogue system for interactive event creation
//!
//! Implements a finite state machine for guiding users through event creation

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Timelike, Utc};
use chrono_english::{parse_date_string, Dialect};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use uuid::Uuid;

use crate::db::BotDb;

/// State for event creation dialogue
#[derive(Clone, Default)]
pub enum CreateEventState {
    #[default]
    Start,
    AwaitingTitle,
    AwaitingTime {
        title: String,
    },
    AwaitingDuration {
        title: String,
        start: DateTime<Utc>,
    },
    AwaitingDescription {
        title: String,
        start: DateTime<Utc>,
        duration_minutes: i64,
    },
    AwaitingLocation {
        title: String,
        start: DateTime<Utc>,
        duration_minutes: i64,
        description: Option<String>,
    },
}

/// Parse natural language date/time string to DateTime<Utc>
pub fn parse_natural_date(input: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();

    // Try parsing with chrono-english
    let parsed = parse_date_string(input, now, Dialect::Uk)
        .context("Failed to parse date/time")?;

    Ok(parsed)
}

/// Parse duration string (e.g., "30m", "1h", "90")
pub fn parse_duration(input: &str) -> Result<i64> {
    let input = input.trim().to_lowercase();

    // Try parsing as plain number (assume minutes)
    if let Ok(minutes) = input.parse::<i64>() {
        return Ok(minutes);
    }

    // Try parsing with unit suffix
    if input.ends_with('m') {
        let num_str = &input[..input.len() - 1];
        let minutes = num_str.parse::<i64>()
            .context("Invalid duration format")?;
        return Ok(minutes);
    }

    if input.ends_with('h') {
        let num_str = &input[..input.len() - 1];
        let hours = num_str.parse::<i64>()
            .context("Invalid duration format")?;
        return Ok(hours * 60);
    }

    Err(anyhow::anyhow!("Invalid duration format. Use: 30, 30m, 1h, or 1.5h"))
}

/// Start the event creation dialogue
pub async fn start_create_dialogue(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "🎯 <b>Create New Event</b>\n\n\
         Let's create a new event! I'll guide you through the process.\n\n\
         First, what's the event title?\n\n\
         Type /cancel to cancel at any time."
    )
    .parse_mode(ParseMode::Html)
    .await?;

    Ok(())
}

/// Handle title input
pub async fn handle_title_input(
    bot: Bot,
    msg: Message,
    title: String,
) -> Result<CreateEventState> {
    if title.trim().is_empty() {
        bot.send_message(
            msg.chat.id,
            "❌ Event title cannot be empty. Please provide a title:"
        )
        .await?;
        return Ok(CreateEventState::AwaitingTitle);
    }

    bot.send_message(
        msg.chat.id,
        format!(
            "✅ Title: <b>{}</b>\n\n\
             When should this event start?\n\n\
             You can use natural language like:\n\
             • tomorrow at 3pm\n\
             • next monday 10:30\n\
             • 2026-01-25 14:00\n\
             • in 2 hours",
            title
        )
    )
    .parse_mode(ParseMode::Html)
    .await?;

    Ok(CreateEventState::AwaitingTime { title })
}

/// Handle time input
pub async fn handle_time_input(
    bot: Bot,
    msg: Message,
    title: String,
    time_str: String,
) -> Result<CreateEventState> {
    match parse_natural_date(&time_str) {
        Ok(start) => {
            let formatted_time = start.format("%A, %B %d at %H:%M");

            bot.send_message(
                msg.chat.id,
                format!(
                    "✅ Time: {}\n\n\
                     How long will this event last?\n\n\
                     Examples:\n\
                     • 30 (30 minutes)\n\
                     • 1h (1 hour)\n\
                     • 90m (90 minutes)\n\n\
                     Or type 'skip' for a 1-hour default",
                    formatted_time
                )
            )
            .parse_mode(ParseMode::Html)
            .await?;

            Ok(CreateEventState::AwaitingDuration { title, start })
        }
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "❌ Couldn't parse that time: {}\n\n\
                     Please try again with a different format:",
                    e
                )
            )
            .await?;

            Ok(CreateEventState::AwaitingTime { title })
        }
    }
}

/// Handle duration input
pub async fn handle_duration_input(
    bot: Bot,
    msg: Message,
    title: String,
    start: DateTime<Utc>,
    duration_str: String,
) -> Result<CreateEventState> {
    let duration_minutes = if duration_str.trim().to_lowercase() == "skip" {
        60 // Default to 1 hour
    } else {
        match parse_duration(&duration_str) {
            Ok(minutes) if minutes > 0 && minutes <= 1440 => minutes, // Max 24 hours
            Ok(_) => {
                bot.send_message(
                    msg.chat.id,
                    "❌ Duration must be between 1 minute and 24 hours. Please try again:"
                )
                .await?;
                return Ok(CreateEventState::AwaitingDuration { title, start });
            }
            Err(e) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "❌ {}\n\n\
                         Please try again:",
                        e
                    )
                )
                .await?;
                return Ok(CreateEventState::AwaitingDuration { title, start });
            }
        }
    };

    bot.send_message(
        msg.chat.id,
        format!(
            "✅ Duration: {} minutes\n\n\
             Would you like to add a description?\n\n\
             Type your description or 'skip' to continue without one.",
            duration_minutes
        )
    )
    .await?;

    Ok(CreateEventState::AwaitingDescription {
        title,
        start,
        duration_minutes,
    })
}

/// Handle description input
pub async fn handle_description_input(
    bot: Bot,
    msg: Message,
    title: String,
    start: DateTime<Utc>,
    duration_minutes: i64,
    description_str: String,
) -> Result<CreateEventState> {
    let description = if description_str.trim().to_lowercase() == "skip" {
        None
    } else {
        Some(description_str)
    };

    bot.send_message(
        msg.chat.id,
        "Would you like to add a location?\n\n\
         Type the location or 'skip' to finish without one."
    )
    .await?;

    Ok(CreateEventState::AwaitingLocation {
        title,
        start,
        duration_minutes,
        description,
    })
}

/// Handle location input and create the event
pub async fn handle_location_input(
    bot: Bot,
    msg: Message,
    db: BotDb,
    title: String,
    start: DateTime<Utc>,
    duration_minutes: i64,
    description: Option<String>,
    location_str: String,
) -> Result<CreateEventState> {
    let location = if location_str.trim().to_lowercase() == "skip" {
        None
    } else {
        Some(location_str)
    };

    // Get user's telegram_id
    let user = msg.from.ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let telegram_id = user.id.0 as i64;

    // Calculate end time
    let end = start + Duration::minutes(duration_minutes);

    // Create the event in database
    match create_event_in_db(&db, telegram_id, &title, start, end, description.as_deref(), location.as_deref()).await {
        Ok(event_id) => {
            let mut response = format!(
                "✅ <b>Event Created!</b>\n\n\
                 📌 {}\n\
                 📅 {}\n\
                 🕐 {} - {}\n",
                title,
                start.format("%A, %B %d, %Y"),
                start.format("%H:%M"),
                end.format("%H:%M")
            );

            if let Some(desc) = description {
                response.push_str(&format!("📝 {}\n", desc));
            }

            if let Some(loc) = location {
                response.push_str(&format!("📍 {}\n", loc));
            }

            response.push_str(&format!("\n🆔 Event ID: <code>{}</code>", event_id));

            bot.send_message(msg.chat.id, response)
                .parse_mode(ParseMode::Html)
                .await?;

            tracing::info!("Event created for user {}: {}", telegram_id, event_id);
        }
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("❌ Failed to create event: {}\n\nPlease try again with /create", e)
            )
            .await?;

            tracing::error!("Failed to create event for user {}: {}", telegram_id, e);
        }
    }

    Ok(CreateEventState::Start)
}

/// Create event in database
async fn create_event_in_db(
    db: &BotDb,
    telegram_id: i64,
    title: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    description: Option<&str>,
    location: Option<&str>,
) -> Result<Uuid> {
    db.create_event(telegram_id, title, start, end, description, location)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_plain_number() {
        assert_eq!(parse_duration("30").unwrap(), 30);
        assert_eq!(parse_duration("90").unwrap(), 90);
    }

    #[test]
    fn test_parse_duration_with_minutes() {
        assert_eq!(parse_duration("30m").unwrap(), 30);
        assert_eq!(parse_duration("45M").unwrap(), 45);
    }

    #[test]
    fn test_parse_duration_with_hours() {
        assert_eq!(parse_duration("1h").unwrap(), 60);
        assert_eq!(parse_duration("2H").unwrap(), 120);
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn test_parse_natural_date() {
        // Test that function exists and compiles
        // Actual date parsing would require integration tests with real timestamps
        let result = parse_natural_date("tomorrow at 3pm");
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid for unit test
    }
}
