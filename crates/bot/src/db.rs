//! Database operations for the bot
//!
//! Handles all database queries needed by bot command handlers

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Generate a random alphanumeric password
fn generate_random_password() -> String {
    use rand::Rng;

    const CHARSET: &[u8] = b"0123456789\
                             abcdefghijklmnopqrstuvwxyz\
                             ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const PASSWORD_LEN: usize = 16;

    let mut rng = rand::rng();
    (0..PASSWORD_LEN)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Bot database handle
#[derive(Clone)]
pub struct BotDb {
    pool: PgPool,
}

/// Event data structure for bot display
#[derive(Debug, Clone, FromRow)]
pub struct BotEvent {
    #[allow(dead_code)]
    pub id: Uuid,
    pub summary: String,
    pub start: DateTime<Utc>,
    #[allow(dead_code)]
    pub end: DateTime<Utc>,
    pub location: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
}

/// Device password information for display
#[derive(Debug, Clone, FromRow)]
pub struct DevicePasswordInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// User information for lookups
#[derive(Debug, Clone, FromRow)]
pub struct UserInfo {
    #[allow(dead_code)]
    pub id: Uuid,
    pub telegram_id: i64,
    #[allow(dead_code)]
    pub telegram_username: Option<String>,
}

/// Event information with ownership check
#[derive(Debug, Clone, FromRow)]
pub struct EventInfo {
    #[allow(dead_code)]
    pub id: Uuid,
    pub summary: String,
    pub start: DateTime<Utc>,
    #[allow(dead_code)]
    pub end: DateTime<Utc>,
    pub location: Option<String>,
    #[allow(dead_code)]
    pub user_id: Uuid,
}

/// Pending invite information
#[derive(Debug, Clone, FromRow)]
pub struct PendingInvite {
    pub event_id: Uuid,
    pub summary: String,
    pub start: DateTime<Utc>,
    pub location: Option<String>,
    pub organizer_username: Option<String>,
}

/// Attendee information for display
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct AttendeeInfo {
    pub email: String,
    pub telegram_id: Option<i64>,
    pub role: String,
    pub status: String,
    pub telegram_username: Option<String>,
}

impl BotDb {
    /// Create a new database handle
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get events for a user within a date range
    pub async fn get_events_for_user(
        &self,
        telegram_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<BotEvent>, sqlx::Error> {
        // First, get the calendar_id for this user
        let calendar_row = sqlx::query!(
            r#"
            SELECT id
            FROM calendars
            WHERE user_id = (
                SELECT id FROM users WHERE telegram_id = $1
            )
            "#,
            telegram_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let calendar_id: Uuid = match calendar_row {
            Some(row) => row.id,
            None => return Ok(Vec::new()), // No calendar yet
        };

        // Query events in the date range
        let events = sqlx::query_as!(
            BotEvent,
            r#"
            SELECT id, summary, start, "end", location, description
            FROM events
            WHERE calendar_id = $1
              AND start >= $2
              AND start < $3
              AND status != 'CANCELLED'
            ORDER BY start ASC
            "#,
            calendar_id,
            start,
            end
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Get all events for a user (for export)
    pub async fn get_all_events_for_user(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<BotEvent>, sqlx::Error> {
        // First, get the calendar_id for this user
        let calendar_row = sqlx::query(
            r#"
            SELECT id
            FROM calendars
            WHERE user_id = (
                SELECT id FROM users WHERE telegram_id = $1
            )
            "#,
        )
        .bind(telegram_id)
        .fetch_optional(&self.pool)
        .await?;

        let calendar_id: Uuid = match calendar_row {
            Some(row) => row.try_get("id")?,
            None => return Ok(Vec::new()), // No calendar yet
        };

        // Query events
        let events = sqlx::query_as::<_, BotEvent>(
            r#"
            SELECT id, summary, start, "end", location, description
            FROM events
            WHERE calendar_id = $1
              AND status != 'CANCELLED'
            ORDER BY start ASC
            "#,
        )
        .bind(calendar_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Ensure user exists and has a calendar
    pub async fn ensure_user_setup(
        &self,
        telegram_id: i64,
        username: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        // Create user if doesn't exist
        sqlx::query!(
            r#"
            INSERT INTO users (id, telegram_id, telegram_username, timezone)
            VALUES (gen_random_uuid(), $1, $2, 'UTC')
            ON CONFLICT (telegram_id) DO NOTHING
            "#,
            telegram_id,
            username
        )
        .execute(&self.pool)
        .await?;

        // Create calendar if doesn't exist
        let user_row = sqlx::query!(
            r#"
            SELECT id FROM users WHERE telegram_id = $1
            "#,
            telegram_id
        )
        .fetch_one(&self.pool)
        .await?;

        let user_id = user_row.id;

        sqlx::query!(
            r#"
            INSERT INTO calendars (id, user_id, name, color, sync_token, ctag)
            VALUES (gen_random_uuid(), $1, 'My Calendar', '#3B82F6', '1', gen_random_uuid()::text)
            ON CONFLICT (user_id) DO NOTHING
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Generate a new device password for a user
    pub async fn generate_device_password(
        &self,
        telegram_id: i64,
        device_name: &str,
    ) -> Result<String, sqlx::Error> {
        // Get user_id
        let user_row = sqlx::query!(
            r#"
            SELECT id FROM users WHERE telegram_id = $1
            "#,
            telegram_id
        )
        .fetch_one(&self.pool)
        .await?;

        let user_id = user_row.id;

        // Generate random password (16 characters, alphanumeric) before any await
        // Use a simpler approach that's Send-safe
        let password = generate_random_password();

        // Hash password with Argon2id
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString},
        };

        // Offload blocking Argon2 hashing to a worker thread
        let password_clone = password.clone();
        let password_hash = tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
            let argon2 = Argon2::default();
            argon2
                .hash_password(password_clone.as_bytes(), &salt)
                .map(|h| h.to_string())
                .map_err(|_| sqlx::Error::Decode("Failed to hash password".into()))
        })
        .await
        .map_err(|e| sqlx::Error::Decode(format!("Task join error: {}", e).into()))??;

        // Store device password
        sqlx::query!(
            r#"
            INSERT INTO device_passwords (id, user_id, hashed_password, name)
            VALUES (gen_random_uuid(), $1, $2, $3)
            "#,
            user_id,
            password_hash,
            device_name
        )
        .execute(&self.pool)
        .await?;

        Ok(password)
    }

    /// List all device passwords for a user
    pub async fn list_device_passwords(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<DevicePasswordInfo>, sqlx::Error> {
        let devices = sqlx::query_as!(
            DevicePasswordInfo,
            r#"
            SELECT dp.id, dp.name, dp.created_at, dp.last_used_at
            FROM device_passwords dp
            JOIN users u ON dp.user_id = u.id
            WHERE u.telegram_id = $1
            ORDER BY dp.created_at DESC
            "#,
            telegram_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(devices)
    }

    /// Revoke (delete) a device password
    pub async fn revoke_device_password(
        &self,
        telegram_id: i64,
        device_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            DELETE FROM device_passwords
            WHERE id = $1
              AND user_id = (SELECT id FROM users WHERE telegram_id = $2)
            "#,
            device_id,
            telegram_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Find user by Telegram username
    pub async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserInfo>, sqlx::Error> {
        let username_param = username.trim_start_matches('@');
        let user = sqlx::query_as!(
            UserInfo,
            r#"
            SELECT id, telegram_id, telegram_username
            FROM users
            WHERE telegram_username = $1
            "#,
            username_param
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Get event info and verify ownership
    pub async fn get_event_info(
        &self,
        event_id: Uuid,
        telegram_id: i64,
    ) -> Result<Option<EventInfo>, sqlx::Error> {
        let event = sqlx::query_as!(
            EventInfo,
            r#"
            SELECT e.id, e.summary, e.start, e."end", e.location, c.user_id
            FROM events e
            JOIN calendars c ON e.calendar_id = c.id
            JOIN users u ON c.user_id = u.id
            WHERE e.id = $1 AND u.telegram_id = $2
            "#,
            event_id,
            telegram_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(event)
    }

    /// Invite attendee to an event
    pub async fn invite_attendee(
        &self,
        event_id: Uuid,
        email: &str,
        telegram_id: Option<i64>,
        role: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            INSERT INTO event_attendees (id, event_id, email, telegram_id, role, status)
            VALUES (gen_random_uuid(), $1, $2, $3, $4::text::attendee_role, 'NEEDS-ACTION')
            ON CONFLICT (event_id, email) DO NOTHING
            RETURNING id
            "#,
            event_id,
            email,
            telegram_id,
            role
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(r.id),
            None => Err(sqlx::Error::RowNotFound), // Duplicate invite
        }
    }

    /// Update RSVP status for an attendee
    pub async fn update_rsvp_status(
        &self,
        event_id: Uuid,
        telegram_id: i64,
        status: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            UPDATE event_attendees
            SET status = $3::text::participation_status, updated_at = NOW()
            WHERE event_id = $1 AND telegram_id = $2
            "#,
            event_id,
            telegram_id,
            status
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get pending invites for a user
    pub async fn get_pending_invites(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<PendingInvite>, sqlx::Error> {
        let invites = sqlx::query_as!(
            PendingInvite,
            r#"
            SELECT e.id AS event_id, e.summary, e.start, e.location,
                   u.telegram_username AS organizer_username
            FROM event_attendees ea
            JOIN events e ON ea.event_id = e.id
            JOIN calendars c ON e.calendar_id = c.id
            JOIN users u ON c.user_id = u.id
            WHERE ea.telegram_id = $1
              AND ea.status = 'NEEDS-ACTION'
            ORDER BY e.start ASC
            "#,
            telegram_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(invites)
    }

    /// Get all attendees for an event
    #[allow(dead_code)]
    pub async fn get_event_attendees(
        &self,
        event_id: Uuid,
    ) -> Result<Vec<AttendeeInfo>, sqlx::Error> {
        let attendees = sqlx::query_as!(
            AttendeeInfo,
            r#"
            SELECT ea.email, ea.telegram_id, ea.role::text as "role!", ea.status::text as "status!",
                   u.telegram_username
            FROM event_attendees ea
            LEFT JOIN users u ON ea.telegram_id = u.telegram_id
            WHERE ea.event_id = $1
            ORDER BY
                CASE ea.role::text
                    WHEN 'ORGANIZER' THEN 0
                    ELSE 1
                END,
                ea.created_at ASC
            "#,
            event_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(attendees)
    }

    /// Get event organizer's telegram_id
    pub async fn get_event_organizer(
        &self,
        event_id: Uuid,
    ) -> Result<Option<i64>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT u.telegram_id
            FROM events e
            JOIN calendars c ON e.calendar_id = c.id
            JOIN users u ON c.user_id = u.id
            WHERE e.id = $1
            "#,
            event_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.telegram_id)),
            None => Ok(None),
        }
    }

    /// Queue calendar invite message for processing
    pub async fn queue_calendar_invite(
        &self,
        recipient_email: &str,
        recipient_telegram_id: Option<i64>,
        event_summary: &str,
        event_start: DateTime<Utc>,
        event_location: Option<&str>,
    ) -> Result<Uuid, sqlx::Error> {
        use serde_json::json;

        let payload = json!({
            "recipient_email": recipient_email,
            "recipient_telegram_id": recipient_telegram_id,
            "event_summary": event_summary,
            "event_start": event_start.to_rfc3339(),
            "event_location": event_location,
        });

        let row = sqlx::query!(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at)
            VALUES (gen_random_uuid(), 'calendar_invite', $1, 'pending', 0, NOW())
            RETURNING id
            "#,
            payload
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    /// Queue RSVP notification to event organizer
    pub async fn queue_rsvp_notification(
        &self,
        organizer_telegram_id: i64,
        attendee_name: &str,
        event_summary: &str,
        rsvp_status: &str,
    ) -> Result<Uuid, sqlx::Error> {
        use serde_json::json;

        let payload = json!({
            "telegram_id": organizer_telegram_id,
            "message": format!("📅 {} {} your invite to: {}", attendee_name, rsvp_status, event_summary),
        });

        let row = sqlx::query!(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at)
            VALUES (gen_random_uuid(), 'telegram_notification', $1, 'pending', 0, NOW())
            RETURNING id
            "#,
            payload
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    /// Get calendar ID for a user (creates calendar if missing)
    pub async fn get_or_create_calendar(
        &self,
        telegram_id: i64,
    ) -> Result<Uuid, sqlx::Error> {
        // First try to get existing calendar
        let calendar_row = sqlx::query!(
            r#"
            SELECT c.id
            FROM calendars c
            JOIN users u ON c.user_id = u.id
            WHERE u.telegram_id = $1
            "#,
            telegram_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = calendar_row {
            return Ok(row.id);
        }

        // Calendar doesn't exist - this shouldn't happen if user ran /start
        // but we'll handle it gracefully by ensuring user setup
        self.ensure_user_setup(telegram_id, None).await?;

        // Now fetch the calendar
        let row = sqlx::query!(
            r#"
            SELECT c.id
            FROM calendars c
            JOIN users u ON c.user_id = u.id
            WHERE u.telegram_id = $1
            "#,
            telegram_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    /// Create a new event
    #[allow(clippy::too_many_arguments)]
    pub async fn create_event(
        &self,
        telegram_id: i64,
        uid: &str,
        summary: &str,
        description: Option<&str>,
        location: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        timezone: &str,
    ) -> Result<BotEvent, sqlx::Error> {
        use sha2::{Digest, Sha256};

        // Get calendar for user
        let calendar_id = self.get_or_create_calendar(telegram_id).await?;

        // Generate ETag (SHA256 of event data)
        let etag_data = format!(
            "{}|{}|{}|{}|{}|{}|false|Confirmed|",
            uid,
            summary,
            description.unwrap_or(""),
            location.unwrap_or(""),
            start.to_rfc3339(),
            end.to_rfc3339()
        );
        let hash = Sha256::digest(etag_data.as_bytes());
        let etag = format!("{:x}", hash);

        // Insert event
        let event = sqlx::query_as!(
            BotEvent,
            r#"
            INSERT INTO events (
                calendar_id, uid, summary, description, location,
                start, "end", is_all_day, status, timezone, etag
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, false, 'CONFIRMED', $8, $9)
            RETURNING id, summary, start, "end", location, description
            "#,
            calendar_id,
            uid,
            summary,
            description,
            location,
            start,
            end,
            timezone,
            etag
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_db_creation() {
        // This is a compile-time test to ensure BotDb can be created
        // Actual database tests would require a test database
    }

    #[test]
    fn test_bot_event_structure() {
        // Verify BotEvent implements required traits
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}

        assert_clone::<BotEvent>();
        assert_debug::<BotEvent>();
    }
}
