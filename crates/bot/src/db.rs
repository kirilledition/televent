//! Database operations for the bot
//!
//! Handles all database queries needed by bot command handlers
//!
//! TODO: Once database is set up in dev environment, run `cargo sqlx prepare`
//! to generate offline query cache for compile-time verification.

use chrono::{DateTime, Utc};
use sha2::Digest;
use sqlx::{FromRow, PgPool, Row};
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
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub location: Option<String>,
    pub description: Option<String>,
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

        // Query events in the date range
        let events = sqlx::query_as::<_, BotEvent>(
            r#"
            SELECT id, title, start, "end", location, description
            FROM events
            WHERE calendar_id = $1
              AND start >= $2
              AND start < $3
              AND status != 'cancelled'
            ORDER BY start ASC
            "#,
        )
        .bind(calendar_id)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Get a specific event by ID
    pub async fn get_event(&self, event_id: Uuid) -> Result<Option<BotEvent>, sqlx::Error> {
        let event = sqlx::query_as::<_, BotEvent>(
            r#"
            SELECT id, title, start, "end", location, description
            FROM events
            WHERE id = $1
            "#,
        )
        .bind(event_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(event)
    }

    /// Check if user has a calendar
    pub async fn user_has_calendar(&self, telegram_id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM calendars
                WHERE user_id = (
                    SELECT id FROM users WHERE telegram_id = $1
                )
            ) as exists
            "#,
        )
        .bind(telegram_id)
        .fetch_one(&self.pool)
        .await?;

        let exists: bool = result.try_get("exists")?;
        Ok(exists)
    }

    /// Create a new event for a user
    pub async fn create_event(
        &self,
        telegram_id: i64,
        title: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        description: Option<&str>,
        location: Option<&str>,
    ) -> Result<Uuid, sqlx::Error> {
        // Get user's calendar_id
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
        .fetch_one(&self.pool)
        .await?;

        let calendar_id: Uuid = calendar_row.try_get("id")?;

        // Generate event ID and UID
        let event_id = Uuid::new_v4();
        let uid = format!("{}@televent.app", event_id);

        // Generate ETag (SHA256 of event data)
        let etag_data = format!("{}|{}|{}|{}", uid, title, start, end);
        let etag = format!("{:x}", sha2::Sha256::digest(etag_data.as_bytes()));

        // Create event
        sqlx::query(
            r#"
            INSERT INTO events (
                id, calendar_id, uid, title, description, location,
                start, "end", is_all_day, status, timezone, version, etag
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(event_id)
        .bind(calendar_id)
        .bind(&uid)
        .bind(title)
        .bind(description)
        .bind(location)
        .bind(start)
        .bind(end)
        .bind(false) // is_all_day
        .bind("confirmed") // status
        .bind("UTC") // timezone
        .bind(1_i32) // version
        .bind(etag)
        .execute(&self.pool)
        .await?;

        // Update calendar's ctag
        sqlx::query(
            r#"
            UPDATE calendars
            SET ctag = gen_random_uuid()::text
            WHERE id = $1
            "#,
        )
        .bind(calendar_id)
        .execute(&self.pool)
        .await?;

        Ok(event_id)
    }

    /// Ensure user exists and has a calendar
    pub async fn ensure_user_setup(&self, telegram_id: i64, username: Option<&str>) -> Result<(), sqlx::Error> {
        // Create user if doesn't exist
        sqlx::query(
            r#"
            INSERT INTO users (id, telegram_id, telegram_username, timezone)
            VALUES (gen_random_uuid(), $1, $2, 'UTC')
            ON CONFLICT (telegram_id) DO NOTHING
            "#,
        )
        .bind(telegram_id)
        .bind(username)
        .execute(&self.pool)
        .await?;

        // Create calendar if doesn't exist
        let user_row = sqlx::query(
            r#"
            SELECT id FROM users WHERE telegram_id = $1
            "#,
        )
        .bind(telegram_id)
        .fetch_one(&self.pool)
        .await?;

        let user_id: Uuid = user_row.try_get("id")?;

        sqlx::query(
            r#"
            INSERT INTO calendars (id, user_id, name, color, sync_token, ctag)
            VALUES (gen_random_uuid(), $1, 'My Calendar', '#3B82F6', '1', gen_random_uuid()::text)
            ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
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
        let user_row = sqlx::query(
            r#"
            SELECT id FROM users WHERE telegram_id = $1
            "#,
        )
        .bind(telegram_id)
        .fetch_one(&self.pool)
        .await?;

        let user_id: Uuid = user_row.try_get("id")?;

        // Generate random password (16 characters, alphanumeric) before any await
        // Use a simpler approach that's Send-safe
        let password = generate_random_password();

        // Hash password with Argon2id
        use argon2::{
            password_hash::{PasswordHasher, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| sqlx::Error::Decode("Failed to hash password".into()))?
            .to_string();

        // Store device password
        sqlx::query(
            r#"
            INSERT INTO device_passwords (id, user_id, hashed_password, name)
            VALUES (gen_random_uuid(), $1, $2, $3)
            "#,
        )
        .bind(user_id)
        .bind(&password_hash)
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
            SELECT dp.id, dp.name, dp.created_at, dp.last_used_at
            FROM device_passwords dp
            JOIN users u ON dp.user_id = u.id
            WHERE u.telegram_id = $1
            ORDER BY dp.created_at DESC
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
            WHERE id = $1
              AND user_id = (SELECT id FROM users WHERE telegram_id = $2)
            "#,
        )
        .bind(device_id)
        .bind(telegram_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
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
