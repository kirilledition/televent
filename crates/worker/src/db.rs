//! Database operations for the worker
//!
//! Handles fetching and updating outbox messages

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Row};
use uuid::Uuid;

/// Outbox message from database
#[derive(Debug, Clone, FromRow)]
pub struct OutboxMessage {
    pub id: Uuid,
    pub message_type: String,
    pub payload: Value,
    pub status: String,
    pub retry_count: i32,
    pub scheduled_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

/// Fetch pending messages and mark them as processing
pub async fn fetch_pending_messages(
    pool: &PgPool,
    batch_size: i64,
) -> Result<Vec<OutboxMessage>, sqlx::Error> {
    let messages = sqlx::query_as::<_, OutboxMessage>(
        r#"
        UPDATE outbox_messages
        SET status = 'processing'
        WHERE id IN (
            SELECT id FROM outbox_messages
            WHERE status = 'pending'
              AND scheduled_at <= NOW()
            ORDER BY scheduled_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING *
        "#,
    )
    .bind(batch_size)
    .fetch_all(pool)
    .await?;

    Ok(messages)
}

/// Mark message as completed
pub async fn mark_completed(pool: &PgPool, message_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE outbox_messages
        SET status = 'completed',
            processed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(message_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark message as failed
pub async fn mark_failed(pool: &PgPool, message_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE outbox_messages
        SET status = 'failed',
            processed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(message_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Reschedule message for retry with exponential backoff
pub async fn reschedule_for_retry(
    pool: &PgPool,
    message_id: Uuid,
    retry_count: i32,
) -> Result<(), sqlx::Error> {
    // Exponential backoff: 2^retry_count minutes
    let backoff_minutes = 2_i64.pow(retry_count as u32);
    let next_scheduled = Utc::now() + Duration::minutes(backoff_minutes);

    sqlx::query(
        r#"
        UPDATE outbox_messages
        SET status = 'pending',
            retry_count = $2,
            scheduled_at = $3
        WHERE id = $1
        "#,
    )
    .bind(message_id)
    .bind(retry_count)
    .bind(next_scheduled)
    .execute(pool)
    .await?;

    Ok(())
}

/// Clean up old completed/failed messages (older than 90 days)
pub async fn cleanup_old_messages(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let cutoff = Utc::now() - Duration::days(90);

    let result = sqlx::query(
        r#"
        DELETE FROM outbox_messages
        WHERE (status = 'completed' OR status = 'failed')
          AND processed_at < $1
        "#,
    )
    .bind(cutoff)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
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
}
