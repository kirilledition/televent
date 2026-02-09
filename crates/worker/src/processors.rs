//! Message processors
//!
//! Handles different types of outbox messages

use anyhow::{Context, Result};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::{error, info};

use crate::db::OutboxMessage;
use crate::mailer::Mailer;
use sqlx::PgPool;
use televent_core::attendee::is_internal_email;
use televent_core::models::Event;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use uuid::Uuid;

/// Process a single outbox message
pub async fn process_message(
    pool: &PgPool,
    message: &OutboxMessage,
    bot: &Bot,
    mailer: &Mailer,
) -> Result<()> {
    match message.message_type.as_str() {
        "invite_notification" => process_invite_notification(pool, message, bot).await,
        "telegram_notification" => process_telegram_notification(message, bot).await,
        "email" => process_email(message, mailer).await,
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

/// Process an invite notification
async fn process_invite_notification(
    pool: &PgPool,
    message: &OutboxMessage,
    bot: &Bot,
) -> Result<()> {
    let event_id_str = message.payload["event_id"]
        .as_str()
        .context("Missing event_id in payload")?;
    let event_id = Uuid::parse_str(event_id_str).context("Invalid event_id")?;

    let target_telegram_id = message.payload["target_user_id"]
        .as_i64()
        .context("Missing target_user_id in payload")?;

    // Fetch event details
    let event: Event = sqlx::query_as("SELECT * FROM events WHERE id = $1")
        .bind(event_id)
        .fetch_one(pool)
        .await
        .context("Failed to fetch event")?;

    let time_str = if let Some(start) = event.start {
        start.format("%Y-%m-%d %H:%M UTC").to_string()
    } else if let Some(date) = event.start_date {
        format!("{} (All Day)", date)
    } else {
        "Unknown Time".to_string()
    };

    let location_text = event
        .location
        .as_ref()
        .map(|loc| format!("\n📍 <b>Location:</b> {}", loc))
        .unwrap_or_default();

    let text = format!(
        "📅 <b>New Invite:</b> {}\n🕒 <b>Time:</b> {}{}",
        event.summary, time_str, location_text
    );

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ Accept", format!("rsvp:{}:ACCEPTED", event.id)),
        InlineKeyboardButton::callback("❌ Decline", format!("rsvp:{}:DECLINED", event.id)),
        InlineKeyboardButton::callback("❔ Tentative", format!("rsvp:{}:TENTATIVE", event.id)),
    ]]);

    bot.send_message(ChatId(target_telegram_id), text)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .context("Failed to send invite notification")?;

    info!(
        "Sent invite notification to user {} for event {} (message: {})",
        target_telegram_id, event.id, message.id
    );

    Ok(())
}

/// Process an email message
async fn process_email(message: &OutboxMessage, mailer: &Mailer) -> Result<()> {
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
    mailer
        .send(to, subject, body)
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
    use super::*;
    use crate::Config;
    use serde_json::json;
    use std::time::Duration;
    use televent_core::config::CoreConfig;
    use televent_core::models::OutboxStatus;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;
    use uuid::Uuid;

    fn create_test_config(port: u16) -> Config {
        Config {
            core: CoreConfig {
                database_url: "postgres://localhost".to_string(),
                telegram_bot_token: "test_token".to_string(),
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "127.0.0.1".to_string(),
            smtp_port: port,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "test@televent.app".to_string(),
            smtp_pool_size: 0,
        }
    }

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

    #[sqlx::test]
    async fn test_process_email_integration(pool: PgPool) {
        // Setup mock SMTP
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(socket);
            let mut line = String::new();

            // Handshake
            reader
                .get_mut()
                .write_all(b"220 localhost ESMTP\r\n")
                .await
                .unwrap();

            loop {
                line.clear();
                let n = reader.read_line(&mut line).await.unwrap();
                if n == 0 {
                    break;
                }

                let cmd = line.split_whitespace().next().unwrap_or("").to_uppercase();
                match cmd.as_str() {
                    "EHLO" | "HELO" => {
                        reader
                            .get_mut()
                            .write_all(b"250-localhost\r\n250 8BITMIME\r\n")
                            .await
                            .unwrap();
                    }
                    "MAIL" => {
                        reader
                            .get_mut()
                            .write_all(b"250 2.1.0 Ok\r\n")
                            .await
                            .unwrap();
                    }
                    "RCPT" => {
                        reader
                            .get_mut()
                            .write_all(b"250 2.1.5 Ok\r\n")
                            .await
                            .unwrap();
                    }
                    "DATA" => {
                        reader
                            .get_mut()
                            .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                            .await
                            .unwrap();
                        loop {
                            line.clear();
                            let n = reader.read_line(&mut line).await.unwrap();
                            if n == 0 || line == ".\r\n" || line == ".\n" {
                                break;
                            }
                        }
                        reader
                            .get_mut()
                            .write_all(b"250 2.0.0 Ok: queued\r\n")
                            .await
                            .unwrap();
                    }
                    "QUIT" => {
                        reader
                            .get_mut()
                            .write_all(b"221 2.0.0 Bye\r\n")
                            .await
                            .unwrap();
                        break;
                    }
                    _ => {
                        reader
                            .get_mut()
                            .write_all(b"500 Command not recognized\r\n")
                            .await
                            .unwrap();
                    }
                }
            }
        });

        // Create message
        let tx_msg = OutboxMessage {
            id: Uuid::new_v4(),
            message_type: "email".to_string(),
            payload: json!({
                "to": "recipient@example.com",
                "subject": "Integration Test",
                "body": "Body content"
            }),
            status: OutboxStatus::Processing,
            retry_count: 0,
            scheduled_at: chrono::Utc::now(),
            processed_at: None,
        };

        // Dummy bot (not used for emails)
        let bot = Bot::new("dummy_token");

        // Config with unique port
        let config = create_test_config(port);
        let mailer = Mailer::new(&config).expect("Failed to create mailer");

        // Give server time to bind
        tokio::time::sleep(Duration::from_millis(50)).await;

        process_message(&pool, &tx_msg, &bot, &mailer)
            .await
            .expect("Failed to process email");

        server.await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_process_invite_notification(pool: PgPool) -> sqlx::Result<()> {
        use televent_core::models::{EventStatus, UserId};

        let bot = Bot::new("token");
        let config = create_test_config(2525);
        let mailer = Mailer::new(&config).expect("Failed to create mailer");

        // Insert Test User
        let user_id = UserId::new(123456789);
        sqlx::query(
            "INSERT INTO users (telegram_id, timezone, sync_token, ctag, created_at, updated_at) 
             VALUES ($1, 'UTC', '0', '0', NOW(), NOW())",
        )
        .bind(user_id)
        .execute(&pool)
        .await?;

        // Insert Test Event
        let event_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO events (id, user_id, uid, summary, start, \"end\", status, timezone, version, etag, created_at, updated_at)
             VALUES ($1, $2, 'uid', 'Test Event', NOW(), NOW() + interval '1 hour', $3, 'UTC', 1, 'etag', NOW(), NOW())"
        )
        .bind(event_id)
        .bind(user_id)
        .bind(EventStatus::Confirmed)
        .execute(&pool)
        .await?;

        // Create Outbox Message
        let msg_id = Uuid::new_v4();
        let payload = json!({
            "event_id": event_id.to_string(),
            "target_user_id": 987654321
        });

        let message = OutboxMessage {
            id: msg_id,
            message_type: "invite_notification".to_string(),
            payload,
            status: OutboxStatus::Processing,
            retry_count: 0,
            scheduled_at: chrono::Utc::now(),
            processed_at: None,
        };

        // Attempt to process
        // This will likely fail due to network error (Bot API),
        // but we verify it reaches that point (authenticating the ID fetching logic)
        let result = process_message(&pool, &message, &bot, &mailer).await;

        // Assert error is present and related to Telegram API failure
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Failed to send invite notification")
        );

        Ok(())
    }
}
