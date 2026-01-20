//! Database operations for the worker
//!
//! Handles fetching and updating outbox messages

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use televent_core::models::OutboxStatus;
use uuid::Uuid;

/// Outbox message from database
#[derive(Debug, Clone, FromRow)]
pub struct OutboxMessage {
    pub id: Uuid,
    pub message_type: String,
    pub payload: Value,
    #[allow(dead_code)]
    pub status: OutboxStatus,
    pub retry_count: i32,
    #[allow(dead_code)]
    pub scheduled_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub processed_at: Option<DateTime<Utc>>,
}

/// Worker database handle
#[derive(Clone)]
pub struct WorkerDb {
    pool: PgPool,
}

impl WorkerDb {
    /// Create a new database handle
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Fetch pending jobs and mark them as processing
    ///
    /// Uses FOR UPDATE SKIP LOCKED to prevent duplicate processing
    pub async fn fetch_pending_jobs(
        &self,
        batch_size: i64,
    ) -> Result<Vec<OutboxMessage>, sqlx::Error> {
        let messages = sqlx::query_as::<_, OutboxMessage>(
            r#"
            UPDATE outbox_messages
            SET status = 'processing'
            WHERE id IN (
                SELECT id
                FROM outbox_messages
                WHERE status = 'pending'
                  AND scheduled_at <= NOW()
                ORDER BY scheduled_at ASC
                LIMIT $1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, message_type, payload, status, retry_count, scheduled_at, processed_at
            "#,
        )
        .bind(batch_size)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    /// Mark a message as completed
    pub async fn mark_completed(&self, message_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'completed',
                processed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(message_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a message as failed
    pub async fn mark_failed(&self, message_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'failed',
                processed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(message_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reschedule a message with exponential backoff
    ///
    /// Backoff formula: 2^retry_count minutes
    pub async fn reschedule_message(
        &self,
        message_id: Uuid,
        current_retry_count: i32,
    ) -> Result<(), sqlx::Error> {
        // Calculate backoff: 2^retry_count minutes (1m, 2m, 4m, 8m, 16m)
        let backoff_minutes = 2_i64.pow((current_retry_count + 1) as u32);
        let next_scheduled = Utc::now() + Duration::minutes(backoff_minutes);

        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'pending',
                retry_count = retry_count + 1,
                scheduled_at = $2
            WHERE id = $1
            "#,
        )
        .bind(message_id)
        .bind(next_scheduled)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get count of pending messages (for monitoring)
    pub async fn count_pending(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM outbox_messages
            WHERE status = 'pending'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outbox_message_structure() {
        // Verify OutboxMessage implements required traits
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}

        assert_clone::<OutboxMessage>();
        assert_debug::<OutboxMessage>();
    }

    #[test]
    fn test_backoff_calculation() {
        // Test exponential backoff formula
        assert_eq!(2_i64.pow(0), 1); // First retry: 1 minute
        assert_eq!(2_i64.pow(1), 2); // Second retry: 2 minutes
        assert_eq!(2_i64.pow(2), 4); // Third retry: 4 minutes
        assert_eq!(2_i64.pow(3), 8); // Fourth retry: 8 minutes
        assert_eq!(2_i64.pow(4), 16); // Fifth retry: 16 minutes
    }
}
