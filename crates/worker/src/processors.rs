//! Message processors
//!
//! Handles different types of outbox messages

use anyhow::{Context, Result};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::{error, info};

use crate::db::OutboxMessage;
use crate::mailer;
use televent_core::attendee::is_internal_email;

/// Process a single outbox message
pub async fn process_message(message: &OutboxMessage, bot: &Bot) -> Result<()> {
    match message.message_type.as_str() {
        "telegram_notification" => process_telegram_notification(message, bot).await,
        "email" => process_email(message).await,
        "calendar_invite" => process_calendar_invite(message, bot).await,
        other => {
            error!("Unknown message type: {}", other);
            Err(anyhow::anyhow!("Unknown message type: {}", other))
        }
    }
}

/// Process a Telegram notification
async fn process_telegram_notification(message: &OutboxMessage, bot: &Bot) -> Result<()> {
    let telegram_id: i64 = message.payload["telegram_id"]
        .as_i64()
        .context("Missing telegram_id in payload")?;

    let text = message.payload["message"]
        .as_str()
        .context("Missing message in payload")?;

    bot.send_message(ChatId(telegram_id), text)
        .await
        .context("Failed to send Telegram message")?;

    info!(
        "Sent Telegram notification to user {} (message: {})",
        telegram_id, message.id
    );

    Ok(())
}

/// Process an email message
async fn process_email(message: &OutboxMessage) -> Result<()> {
    let to = message.payload["to"]
        .as_str()
        .context("Missing 'to' in email payload")?;

    let subject = message.payload["subject"]
        .as_str()
        .context("Missing 'subject' in email payload")?;

    let body = message.payload["body"]
        .as_str()
        .context("Missing 'body' in email payload")?;

    // Use mailer crate to send email
    mailer::send_email(to, subject, body)
        .await
        .context("Failed to send email")?;

    info!(
        "Sent email to {} with subject '{}' (message: {})",
        to, subject, message.id
    );

    Ok(())
}

/// Process a calendar invite (The Interceptor)
///
/// Routes internal emails (@televent.internal) to Telegram notifications,
/// logs external emails (MVP mode - no SMTP sending yet)
async fn process_calendar_invite(message: &OutboxMessage, bot: &Bot) -> Result<()> {
    let recipient_email = message.payload["recipient_email"]
        .as_str()
        .context("Missing recipient_email in payload")?;

    let event_summary = message.payload["event_summary"]
        .as_str()
        .context("Missing event_summary in payload")?;

    let event_start = message.payload["event_start"]
        .as_str()
        .context("Missing event_start in payload")?;

    let event_location = message.payload["event_location"].as_str();

    // THE INTERCEPTOR LOGIC
    if is_internal_email(recipient_email) {
        // Route to Telegram
        let telegram_id = message.payload["recipient_telegram_id"]
            .as_i64()
            .context("Missing recipient_telegram_id for internal invite")?;

        let location_text = event_location
            .map(|loc| format!("\n📍 <b>Location:</b> {}", loc))
            .unwrap_or_default();

        let invite_text = format!(
            "📅 <b>Calendar Invite</b>\n\n\
             <b>Event:</b> {}\n\
             🕒 <b>Time:</b> {}{}\n\n\
             You've been invited to this event. Use /rsvp to respond.",
            event_summary, event_start, location_text
        );

        bot.send_message(ChatId(telegram_id), invite_text)
            .parse_mode(ParseMode::Html)
            .await
            .context("Failed to send Telegram invite notification")?;

        info!(
            "Sent internal invite via Telegram to user {} for event '{}' (message: {})",
            telegram_id, event_summary, message.id
        );
    } else {
        // External invite - log skip (MVP mode)
        info!(
            "External invite skipped (MVP Mode): {} - event '{}' (message: {})",
            recipient_email, event_summary, message.id
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_telegram_notification_payload_parsing() {
        let payload = json!({
            "telegram_id": 123456,
            "message": "Test message"
        });

        assert_eq!(payload["telegram_id"].as_i64(), Some(123456));
        assert_eq!(payload["message"].as_str(), Some("Test message"));
    }

    #[test]
    fn test_email_payload_parsing() {
        let payload = json!({
            "to": "test@example.com",
            "subject": "Test Subject",
            "body": "Test body"
        });

        assert_eq!(payload["to"].as_str(), Some("test@example.com"));
        assert_eq!(payload["subject"].as_str(), Some("Test Subject"));
        assert_eq!(payload["body"].as_str(), Some("Test body"));
    }

    #[test]
    fn test_calendar_invite_payload_parsing() {
        let payload = json!({
            "recipient_email": "tg_123456789@televent.internal",
            "recipient_telegram_id": 123456789,
            "event_summary": "Team Meeting",
            "event_start": "2026-01-21T15:00:00Z",
            "event_location": "Conference Room A"
        });

        assert_eq!(
            payload["recipient_email"].as_str(),
            Some("tg_123456789@televent.internal")
        );
        assert_eq!(payload["recipient_telegram_id"].as_i64(), Some(123456789));
        assert_eq!(payload["event_summary"].as_str(), Some("Team Meeting"));
        assert_eq!(
            payload["event_start"].as_str(),
            Some("2026-01-21T15:00:00Z")
        );
        assert_eq!(
            payload["event_location"].as_str(),
            Some("Conference Room A")
        );
    }
}
