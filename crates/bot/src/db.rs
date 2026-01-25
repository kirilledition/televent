//! Database operations for the bot
//!
//! Handles all database queries needed by bot command handlers

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Row};
use televent_core::models::UserId;
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
    pub id: Uuid,
    pub summary: String,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub description: Option<String>,
}

impl BotEvent {
    /// Get a unified start time for display/sorting
    pub fn display_start(&self) -> DateTime<Utc> {
        if self.is_all_day {
            self.start_date
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .unwrap()
                .and_utc()
        } else {
            self.start.unwrap_or_else(Utc::now)
        }
    }

    /// Get timing as ParsedTiming enum
    pub fn timing(&self) -> crate::event_parser::ParsedTiming {
        if self.is_all_day {
            crate::event_parser::ParsedTiming::AllDay {
                date: self.start_date.unwrap_or_else(|| Utc::now().date_naive()),
            }
        } else {
            // Calculate duration in minutes
            let duration_minutes = if let (Some(s), Some(e)) = (self.start, self.end) {
                (e - s).num_minutes() as u32
            } else {
                60
            };
            crate::event_parser::ParsedTiming::Timed {
                start: self.start.unwrap_or_else(Utc::now),
                duration_minutes,
            }
        }
    }
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
    pub telegram_id: i64,
    #[allow(dead_code)]
    pub telegram_username: Option<String>,
}

/// Event information with ownership check
#[derive(Debug, Clone, FromRow)]
pub struct EventInfo {
    pub id: Uuid,
    pub summary: String,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub user_id: UserId,
}

/// Pending invite information
#[derive(Debug, Clone, FromRow)]
pub struct PendingInvite {
    pub event_id: Uuid,
    pub summary: String,
    pub start: Option<DateTime<Utc>>,
    pub start_date: Option<chrono::NaiveDate>,
    pub is_all_day: bool,
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
        start_range: DateTime<Utc>,
        end_range: DateTime<Utc>,
    ) -> Result<Vec<BotEvent>, sqlx::Error> {
        let start_date = start_range.date_naive();
        let end_date = end_range.date_naive();

        // Query both timed and all-day events
        let events = sqlx::query_as::<_, BotEvent>(
            r#"
            SELECT id, summary, start, "end", start_date, end_date, is_all_day, location, description
            FROM events
            WHERE user_id = $1
              AND (
                  (is_all_day = false AND start >= $2 AND start < $3)
                  OR 
                  (is_all_day = true AND start_date >= $4 AND start_date < $5)
              )
              AND status != 'CANCELLED'
            ORDER BY COALESCE(start, (start_date AT TIME ZONE 'UTC')) ASC
            "#,
        )
        .bind(telegram_id)
        .bind(start_range)
        .bind(end_range)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Get all events for a user (for export)
    pub async fn get_all_events_for_user(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<BotEvent>, sqlx::Error> {
        // Query events directly by user_id
        let events = sqlx::query_as::<_, BotEvent>(
            r#"
            SELECT id, summary, start, "end", start_date, end_date, is_all_day, location, description
            FROM events
            WHERE user_id = $1
              AND status != 'CANCELLED'
            ORDER BY COALESCE(start, (start_date AT TIME ZONE 'UTC')) ASC
            "#,
        )
        .bind(telegram_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Ensure user exists (user = calendar in new schema)
    pub async fn ensure_user_setup(
        &self,
        telegram_id: i64,
        username: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        // Create user if doesn't exist - user now IS the calendar
        sqlx::query(
            r#"
            INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag)
            VALUES ($1, $2, 'UTC', '1', gen_random_uuid()::text)
            ON CONFLICT (telegram_id) DO UPDATE SET
                telegram_username = COALESCE(EXCLUDED.telegram_username, users.telegram_username)
            "#,
        )
        .bind(telegram_id)
        .bind(username)
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
        // Ensure user exists first
        self.ensure_user_setup(telegram_id, None).await?;

        // Generate random password (16 characters, alphanumeric) before any await
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

        // Store device password - user_id is now telegram_id
        sqlx::query(
            r#"
            INSERT INTO device_passwords (id, user_id, password_hash, device_name)
            VALUES (gen_random_uuid(), $1, $2, $3)
            "#,
        )
        .bind(telegram_id)
        .bind(password_hash)
        .bind(device_name)
        .execute(&self.pool)
        .await?;

        Ok(password)
    }

    /// List all device passwords for a user
    pub async fn list_device_passwords(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<DevicePasswordInfo>, sqlx::Error> {
        let devices = sqlx::query_as::<_, DevicePasswordInfo>(
            r#"
            SELECT id, device_name as name, created_at, last_used_at
            FROM device_passwords
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(telegram_id)
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
        let result = sqlx::query(
            r#"
            DELETE FROM device_passwords
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(device_id)
        .bind(telegram_id)
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
        let user = sqlx::query_as::<_, UserInfo>(
            r#"
            SELECT telegram_id, telegram_username
            FROM users
            WHERE lower(telegram_username) = lower($1)
            "#,
        )
        .bind(username_param)
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
        let event = sqlx::query_as::<_, EventInfo>(
            r#"
            SELECT id, summary, start, "end", start_date, end_date, is_all_day, location, user_id
            FROM events
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(event_id)
        .bind(telegram_id)
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
        let row = sqlx::query(
            r#"
            INSERT INTO event_attendees (id, event_id, email, telegram_id, role, status)
            VALUES (gen_random_uuid(), $1, $2, $3, $4::text::attendee_role, 'NEEDS-ACTION')
            ON CONFLICT (event_id, email) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(event_id)
        .bind(email)
        .bind(telegram_id)
        .bind(role)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(r.try_get("id")?),
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
        let result = sqlx::query(
            r#"
            UPDATE event_attendees
            SET status = $3::text::participation_status, updated_at = NOW()
            WHERE event_id = $1 AND telegram_id = $2
            "#,
        )
        .bind(event_id)
        .bind(telegram_id)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get pending invites for a user
    pub async fn get_pending_invites(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<PendingInvite>, sqlx::Error> {
        let invites = sqlx::query_as::<_, PendingInvite>(
            r#"
            SELECT e.id AS event_id, e.summary, e.start, e.start_date, e.is_all_day, e.location,
                   u.telegram_username AS organizer_username
            FROM event_attendees ea
            JOIN events e ON ea.event_id = e.id
            JOIN users u ON e.user_id = u.telegram_id
            WHERE ea.telegram_id = $1
              AND ea.status = 'NEEDS-ACTION'
            ORDER BY COALESCE(e.start, (e.start_date AT TIME ZONE 'UTC')) ASC
            "#,
        )
        .bind(telegram_id)
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
        let attendees = sqlx::query_as::<_, AttendeeInfo>(
            r#"
            SELECT ea.email, ea.telegram_id, ea.role::text as role, ea.status::text as status,
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
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(attendees)
    }

    /// Get event organizer's telegram_id
    pub async fn get_event_organizer(&self, event_id: Uuid) -> Result<Option<i64>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT user_id as telegram_id
            FROM events
            WHERE id = $1
            "#,
        )
        .bind(event_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let tid: i64 = r.try_get("telegram_id")?;
                Ok(Some(tid))
            }
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

        let row = sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at)
            VALUES (gen_random_uuid(), 'calendar_invite', $1, 'pending', 0, NOW())
            RETURNING id
            "#,
        )
        .bind(payload)
        .fetch_one(&self.pool)
        .await?;

        row.try_get("id")
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

        let row = sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at)
            VALUES (gen_random_uuid(), 'telegram_notification', $1, 'pending', 0, NOW())
            RETURNING id
            "#,
        )
        .bind(payload)
        .fetch_one(&self.pool)
        .await?;

        row.try_get("id")
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
        timing: crate::event_parser::ParsedTiming,
        timezone: &str,
    ) -> Result<BotEvent, sqlx::Error> {
        use sha2::{Digest, Sha256};

        // Ensure user exists
        self.ensure_user_setup(telegram_id, None).await?;

        let (start, end, start_date, end_date, is_all_day) = match timing {
            crate::event_parser::ParsedTiming::Timed {
                start,
                duration_minutes,
            } => {
                let end = start + chrono::Duration::minutes(i64::from(duration_minutes));
                (Some(start), Some(end), None, None, false)
            }
            crate::event_parser::ParsedTiming::AllDay { date } => {
                let end_date = date + chrono::Duration::days(1);
                (None, None, Some(date), Some(end_date), true)
            }
        };

        // Generate ETag (SHA256 of event data)
        let etag_data = format!(
            "{}|{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{}|Confirmed|",
            uid,
            summary,
            description.unwrap_or(""),
            location.unwrap_or(""),
            start,
            end,
            start_date,
            end_date,
            is_all_day
        );
        let hash = Sha256::digest(etag_data.as_bytes());
        let etag = format!("{:x}", hash);

        // Insert event - user_id is telegram_id directly
        let event = sqlx::query_as::<_, BotEvent>(
            r#"
            INSERT INTO events (
                user_id, uid, summary, description, location,
                start, "end", start_date, end_date, is_all_day, status, timezone, etag
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'CONFIRMED', $11, $12)
            RETURNING id, summary, start, "end", start_date, end_date, is_all_day, location, description
            "#,
        )
        .bind(telegram_id)
        .bind(uid)
        .bind(summary)
        .bind(description)
        .bind(location)
        .bind(start)
        .bind(end)
        .bind(start_date)
        .bind(end_date)
        .bind(is_all_day)
        .bind(timezone)
        .bind(etag)
        .fetch_one(&self.pool)
        .await?;

        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_user_setup(pool: PgPool) {
        let db = BotDb::new(pool);
        let telegram_id = 1001;

        // Ensure user is set up
        let result = db.ensure_user_setup(telegram_id, Some("testuser")).await;
        assert!(result.is_ok());

        // Ensure idempotency
        let result2 = db.ensure_user_setup(telegram_id, Some("testuser")).await;
        assert!(result2.is_ok());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_event_lifecycle(pool: PgPool) {
        let db = BotDb::new(pool);
        let telegram_id = 1002;
        db.ensure_user_setup(telegram_id, None)
            .await
            .expect("Failed setup");

        let start = Utc::now();
        let uid = format!("{}", Uuid::new_v4());

        // Create event
        let event = db
            .create_event(
                telegram_id,
                &uid,
                "Test Event",
                Some("Description"),
                Some("Location"),
                crate::event_parser::ParsedTiming::Timed {
                    start,
                    duration_minutes: 60,
                },
                "UTC",
            )
            .await
            .expect("Failed to create event");

        assert_eq!(event.summary, "Test Event");
        assert_eq!(event.location.as_deref(), Some("Location"));

        // Retrieve event via get_events_for_user (checking range)
        let events = db
            .get_events_for_user(
                telegram_id,
                start - Duration::minutes(10),
                start + Duration::hours(2),
            )
            .await
            .expect("Failed to get events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Test Event");

        // Retrieve event via get_all_events_for_user
        let all_events = db
            .get_all_events_for_user(telegram_id)
            .await
            .expect("Failed to get all events");
        assert_eq!(all_events.len(), 1);

        // Retrieve event info
        let info = db
            .get_event_info(event.id, telegram_id)
            .await
            .expect("Failed to get info");
        assert!(info.is_some());
        assert_eq!(info.unwrap().summary, "Test Event");

        // Check non-existent event or wrong user
        let info_none = db
            .get_event_info(event.id, 99999)
            .await
            .expect("Failed to query");
        assert!(info_none.is_none());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_device_password_management(pool: PgPool) {
        let db = BotDb::new(pool);
        let telegram_id = 1003;
        db.ensure_user_setup(telegram_id, None)
            .await
            .expect("Setup failed");

        // Create device password
        let password = db
            .generate_device_password(telegram_id, "Test Device")
            .await
            .expect("Generate failed");
        assert_eq!(password.len(), 16);

        // List passwords
        let devices = db
            .list_device_passwords(telegram_id)
            .await
            .expect("List failed");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Test Device");

        // Revoke password
        let revoked = db
            .revoke_device_password(telegram_id, devices[0].id)
            .await
            .expect("Revoke failed");
        assert!(revoked);

        // Revoke again (should be false)
        let revoked2 = db
            .revoke_device_password(telegram_id, devices[0].id)
            .await
            .expect("Revoke2 failed");
        assert!(!revoked2);

        // List again
        let devices_after = db
            .list_device_passwords(telegram_id)
            .await
            .expect("List failed");
        assert!(devices_after.is_empty());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_invites_and_rsvps(pool: PgPool) {
        let db = BotDb::new(pool);
        let organizer_id = 1004;
        let attendee_id = 1005;

        db.ensure_user_setup(organizer_id, Some("organizer"))
            .await
            .expect("Org setup failed");
        db.ensure_user_setup(attendee_id, Some("attendee"))
            .await
            .expect("Att setup failed");

        // Organizer creates event
        let start = Utc::now();
        let uid = format!("{}", Uuid::new_v4());

        let event = db
            .create_event(
                organizer_id,
                &uid,
                "Party",
                None,
                None,
                crate::event_parser::ParsedTiming::Timed {
                    start,
                    duration_minutes: 60,
                },
                "UTC",
            )
            .await
            .expect("Create event failed");

        // Invite attendee
        let _invite_id = db
            .invite_attendee(
                event.id,
                "attendee@example.com",
                Some(attendee_id),
                "ATTENDEE",
            )
            .await
            .expect("Invite failed");

        // Check pending invites
        let pending = db
            .get_pending_invites(attendee_id)
            .await
            .expect("Get pending failed");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].summary, "Party");

        // RSVPs
        let updated = db
            .update_rsvp_status(event.id, attendee_id, "ACCEPTED")
            .await
            .expect("RSVP failed");
        assert!(updated);

        // Check pending again (should be empty as status changed)
        let pending_after = db
            .get_pending_invites(attendee_id)
            .await
            .expect("Get pending failed");
        assert!(pending_after.is_empty());

        // Get attendees list
        let attendees = db
            .get_event_attendees(event.id)
            .await
            .expect("Get attendees");
        assert!(!attendees.is_empty());
        let att = attendees
            .iter()
            .find(|a| a.telegram_id == Some(attendee_id))
            .expect("Attendee not found");
        assert_eq!(att.status, "ACCEPTED");

        // Get organizer id from event
        let org_id_check = db
            .get_event_organizer(event.id)
            .await
            .expect("Get org")
            .unwrap();
        assert_eq!(org_id_check, organizer_id);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_user_lookup_and_outbox(pool: PgPool) {
        let db = BotDb::new(pool);
        let telegram_id = 1006;
        let username = "someuser";

        db.ensure_user_setup(telegram_id, Some(username))
            .await
            .expect("Setup failed");

        // Lookup user
        let user = db
            .find_user_by_username(username)
            .await
            .expect("Lookup failed");
        assert!(user.is_some());
        assert_eq!(user.unwrap().telegram_id, telegram_id);

        // Queue invite
        let invite_msg_id = db
            .queue_calendar_invite(
                "test@test.com",
                Some(telegram_id),
                "Event",
                Utc::now(),
                None,
            )
            .await
            .expect("Queue invite");
        assert!(!invite_msg_id.is_nil());

        // Queue notification
        let notif_msg_id = db
            .queue_rsvp_notification(telegram_id, "Attendee", "Event", "ACCEPTED")
            .await
            .expect("Queue notif");
        assert!(!notif_msg_id.is_nil());
    }

    #[test]
    fn test_bot_db_creation() {
        // This is a compile-time test to ensure BotDb can be created
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
