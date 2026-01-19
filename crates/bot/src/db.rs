//! Database operations for the bot
//!
//! Handles all database queries needed by bot command handlers
//!
//! TODO: Once database is set up in dev environment, run `cargo sqlx prepare`
//! to generate offline query cache for compile-time verification.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Row};
use uuid::Uuid;

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
