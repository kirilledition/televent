//! Message processors
//!
//! Handles different types of outbox messages

use anyhow::{Context, Result};
use teloxide::prelude::*;
use tracing::{error, info};

use crate::db::OutboxMessage;
use crate::mailer;

/// Process a single outbox message
pub async fn process_message(message: &OutboxMessage, bot: &Bot) -> Result<()> {
    match message.message_type.as_str() {
        "telegram_notification" => process_telegram_notification(message, bot).await,
        "email" => process_email(message).await,
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
}
