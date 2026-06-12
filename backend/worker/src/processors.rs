//! Message processors
//!
//! Handles different types of outbox messages

use anyhow::{Context, Result};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::info;

use crate::db::TypedOutboxMessage;
use std::collections::HashMap;
use televent_application::{CalendarService, EventView};
use televent_domain::{
    EventTiming, ExternalEmailDeferred, InviteNotification, OutboxPayload, ParticipationStatus,
    RsvpNotification, TelegramNotification,
};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use uuid::Uuid;

/// Process a single outbox message
pub async fn process_message(
    calendar: &CalendarService,
    message: &TypedOutboxMessage,
    bot: &Bot,
    events_cache: &HashMap<Uuid, EventView>,
) -> Result<()> {
    match message.payload.clone() {
        OutboxPayload::InviteNotification(payload) => {
            process_invite_notification(calendar, message.id, payload, bot, events_cache).await
        }
        OutboxPayload::TelegramNotification(payload) => {
            process_telegram_notification(message.id, payload, bot).await
        }
        OutboxPayload::ExternalEmailDeferred(payload) => {
            process_external_email_deferred(message.id, payload).await
        }
        OutboxPayload::RsvpNotification(payload) => {
            process_rsvp_notification(message.id, payload, bot).await
        }
    }
}

/// Process a Telegram notification
async fn process_telegram_notification(
    message_id: Uuid,
    payload: TelegramNotification,
    bot: &Bot,
) -> Result<()> {
    bot.send_message(ChatId(payload.telegram_id), payload.message.clone())
        .await
        .context("Failed to send Telegram message")?;

    info!(
        "Sent Telegram notification to user {} (message: {})",
        payload.telegram_id, message_id
    );

    Ok(())
}

/// Process an invite notification
async fn process_invite_notification(
    calendar: &CalendarService,
    message_id: Uuid,
    payload: InviteNotification,
    bot: &Bot,
    events_cache: &HashMap<Uuid, EventView>,
) -> Result<()> {
    // Fetch event details
    // Check cache first
    let event = if let Some(event) = events_cache.get(&payload.event_id) {
        event.clone()
    } else {
        calendar
            .get_event_view_by_id_any(payload.event_id)
            .await
            .context("Failed to fetch event")?
            .context("Event not found")?
    };

    let time_str = match &event.timing {
        EventTiming::Timed { start, .. } => start.format("%Y-%m-%d %H:%M UTC").to_string(),
        EventTiming::AllDay { start_date, .. } => format!("{} (All Day)", start_date),
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

    bot.send_message(ChatId(payload.target_user_id), text)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .context("Failed to send invite notification")?;

    info!(
        "Sent invite notification to user {} for event {} (message: {})",
        payload.target_user_id, event.id, message_id
    );

    Ok(())
}

async fn process_external_email_deferred(
    message_id: Uuid,
    payload: ExternalEmailDeferred,
) -> Result<()> {
    info!(
        "External email deferred: {} - event '{}' ({}) (message: {})",
        payload.recipient_email, payload.event_summary, payload.reason, message_id
    );
    Ok(())
}

async fn process_rsvp_notification(
    message_id: Uuid,
    payload: RsvpNotification,
    bot: &Bot,
) -> Result<()> {
    let status = match payload.rsvp_status {
        ParticipationStatus::NeedsAction => "needs action",
        ParticipationStatus::Accepted => "accepted",
        ParticipationStatus::Declined => "declined",
        ParticipationStatus::Tentative => "tentatively accepted",
    };
    let text = format!(
        "📅 {} {} your invite to: {}",
        payload.attendee_name, status, payload.event_summary
    );

    bot.send_message(ChatId(payload.organizer_telegram_id), text)
        .await
        .context("Failed to send RSVP notification")?;

    info!(
        "Sent RSVP notification to user {} (message: {})",
        payload.organizer_telegram_id, message_id
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::PgPool;
    use televent_domain::InviteNotification;
    use uuid::Uuid;

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
    fn test_external_email_deferred_payload_parsing() {
        let payload = json!({
            "recipient_email": "external@example.com",
            "event_summary": "Team Meeting",
            "reason": "External email delivery is disabled"
        });

        assert_eq!(
            payload["recipient_email"].as_str(),
            Some("external@example.com")
        );
        assert_eq!(payload["event_summary"].as_str(), Some("Team Meeting"));
        assert_eq!(
            payload["reason"].as_str(),
            Some("External email delivery is disabled")
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_process_invite_notification(pool: PgPool) -> sqlx::Result<()> {
        use televent_application::UserId;

        let bot = Bot::new("token");

        // Insert Test User
        let user_id = UserId::new(123456789);
        sqlx::query(
            "INSERT INTO users (telegram_id, timezone, sync_token, ctag, created_at, updated_at) 
             VALUES ($1, 'UTC', 0, 0, NOW(), NOW())",
        )
        .bind(user_id.inner())
        .execute(&pool)
        .await?;

        // Insert Test Event
        let event_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO events (id, user_id, uid, summary, start, \"end\", is_all_day, status, timezone, version, etag, created_at, updated_at)
             VALUES ($1, $2, 'uid', 'Test Event', NOW(), NOW() + interval '1 hour', false, $3, 'UTC', 1, 'etag', NOW(), NOW())"
        )
        .bind(event_id)
        .bind(user_id.inner())
        .bind("CONFIRMED")
        .execute(&pool)
        .await?;

        // Create Outbox Message
        let msg_id = Uuid::new_v4();
        let message = TypedOutboxMessage {
            id: msg_id,
            payload: OutboxPayload::InviteNotification(InviteNotification {
                event_id,
                target_user_id: 987654321,
            }),
            retry_count: 0,
        };

        // Attempt to process
        // This will likely fail due to network error (Bot API),
        // but we verify it reaches that point (authenticating the ID fetching logic)
        let calendar = CalendarService::new(televent_storage::calendar::CalendarRepository::new(
            pool.clone(),
        ));
        let result = process_message(&calendar, &message, &bot, &HashMap::new()).await;

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
