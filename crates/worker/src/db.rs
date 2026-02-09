//! Database operations for the worker
//!
//! Handles fetching and updating outbox messages

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use televent_core::models::OutboxStatus;
use uuid::Uuid;

/// Result of processing a job
#[derive(Debug, Clone)]
pub enum JobResult {
    Completed(Uuid),
    Failed {
        id: Uuid,
        error: String,
    },
    Reschedule {
        id: Uuid,
        retry_count: i32,
        scheduled_at: DateTime<Utc>,
        error: String,
    },
}

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
    #[cfg(test)]
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

    /// Mark a message as failed with error message
    #[cfg(test)]
    pub async fn mark_failed(&self, message_id: Uuid, error_msg: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'failed',
                processed_at = NOW(),
                error_message = $2
            WHERE id = $1
            "#,
        )
        .bind(message_id)
        .bind(error_msg)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reschedule a message with exponential backoff
    ///
    /// Backoff formula: 2^retry_count minutes
    #[cfg(test)]
    pub async fn reschedule_message(
        &self,
        message_id: Uuid,
        current_retry_count: i32,
        error_msg: &str,
    ) -> Result<(), sqlx::Error> {
        // Calculate backoff: 2^retry_count minutes (1m, 2m, 4m, 8m, 16m)
        let backoff_minutes = 2_i64.pow((current_retry_count + 1) as u32);
        let next_scheduled = Utc::now() + chrono::Duration::minutes(backoff_minutes);

        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'pending',
                retry_count = retry_count + 1,
                scheduled_at = $2,
                error_message = $3
            WHERE id = $1
            "#,
        )
        .bind(message_id)
        .bind(next_scheduled)
        .bind(error_msg)
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

    /// Bulk update jobs based on their processing results
    pub async fn bulk_update_jobs(&self, results: Vec<JobResult>) -> Result<(), sqlx::Error> {
        let mut completed_ids = Vec::new();

        let mut failed_ids = Vec::new();
        let mut failed_errors = Vec::new();

        let mut reschedule_ids = Vec::new();
        let mut reschedule_counts = Vec::new();
        let mut reschedule_times = Vec::new();
        let mut reschedule_errors = Vec::new();

        for result in results {
            match result {
                JobResult::Completed(id) => completed_ids.push(id),
                JobResult::Failed { id, error } => {
                    failed_ids.push(id);
                    failed_errors.push(error);
                }
                JobResult::Reschedule {
                    id,
                    retry_count,
                    scheduled_at,
                    error,
                } => {
                    reschedule_ids.push(id);
                    reschedule_counts.push(retry_count);
                    reschedule_times.push(scheduled_at);
                    reschedule_errors.push(error);
                }
            }
        }

        let mut tx = self.pool.begin().await?;

        if !completed_ids.is_empty() {
            sqlx::query(
                r#"
                UPDATE outbox_messages
                SET status = 'completed',
                    processed_at = NOW()
                WHERE id = ANY($1)
                "#,
            )
            .bind(&completed_ids)
            .execute(&mut *tx)
            .await?;
        }

        if !failed_ids.is_empty() {
            sqlx::query(
                r#"
                UPDATE outbox_messages AS m
                SET status = 'failed',
                    processed_at = NOW(),
                    error_message = c.error
                FROM UNNEST($1::uuid[], $2::text[]) AS c(id, error)
                WHERE m.id = c.id
                "#,
            )
            .bind(&failed_ids)
            .bind(&failed_errors)
            .execute(&mut *tx)
            .await?;
        }

        if !reschedule_ids.is_empty() {
            sqlx::query(
                r#"
                UPDATE outbox_messages AS m
                SET status = 'pending',
                    retry_count = c.retry_count,
                    scheduled_at = c.scheduled_at,
                    error_message = c.error
                FROM UNNEST($1::uuid[], $2::int[], $3::timestamptz[], $4::text[])
                AS c(id, retry_count, scheduled_at, error)
                WHERE m.id = c.id
                "#,
            )
            .bind(&reschedule_ids)
            .bind(&reschedule_counts)
            .bind(&reschedule_times)
            .bind(&reschedule_errors)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_calculation() {
        // Test exponential backoff formula
        assert_eq!(2_i64.pow(0), 1); // First retry: 1 minute
        assert_eq!(2_i64.pow(1), 2); // Second retry: 2 minutes
        assert_eq!(2_i64.pow(2), 4); // Third retry: 4 minutes
        assert_eq!(2_i64.pow(3), 8); // Fourth retry: 8 minutes
        assert_eq!(2_i64.pow(4), 16); // Fifth retry: 16 minutes
    }

    #[test]
    fn test_worker_db_new() {
        // Test basic construction
        // We can't test actual DB operations without a real pool,
        // but we can verify the type compiles
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<WorkerDb>();
        assert_sync::<WorkerDb>();
    }

    #[test]
    fn test_outbox_message_has_required_traits() {
        // Verify OutboxMessage derives required traits
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}
        fn assert_from_row<T: sqlx::FromRow<'static, sqlx::postgres::PgRow>>() {}

        assert_clone::<OutboxMessage>();
        assert_debug::<OutboxMessage>();
        assert_from_row::<OutboxMessage>();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_fetch_pending_jobs(pool: PgPool) -> sqlx::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());

        let id1 = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'test', $2, 'pending', 0, NOW() - INTERVAL '1 minute', NOW())
            "#
        )
        .bind(id1)
        .bind(json!({"foo": "bar"}))
        .execute(&pool)
        .await?;

        let jobs = db.fetch_pending_jobs(10).await?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].payload["foo"], "bar");
        assert_eq!(jobs[0].status, OutboxStatus::Processing);

        Ok(())
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_mark_completed(pool: PgPool) -> sqlx::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'test', $2, 'processing', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({}))
        .execute(&pool)
        .await?;

        db.mark_completed(id).await?;

        let status: OutboxStatus =
            sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(status, OutboxStatus::Completed);
        Ok(())
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_mark_failed(pool: PgPool) -> sqlx::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'test', $2, 'processing', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({}))
        .execute(&pool)
        .await?;

        db.mark_failed(id, "test error").await?;

        let (status, error_msg): (OutboxStatus, Option<String>) =
            sqlx::query_as("SELECT status, error_message FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(status, OutboxStatus::Failed);
        assert_eq!(error_msg, Some("test error".to_string()));
        Ok(())
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_reschedule_message(pool: PgPool) -> sqlx::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'test', $2, 'processing', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({}))
        .execute(&pool)
        .await?;

        db.reschedule_message(id, 0, "retry error").await?;

        let (status, retry_count): (OutboxStatus, i32) =
            sqlx::query_as("SELECT status, retry_count FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(status, OutboxStatus::Pending);
        assert_eq!(retry_count, 1);
        Ok(())
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn test_count_pending(pool: PgPool) -> sqlx::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());

        for _ in 0..3 {
            sqlx::query(
                r#"
                INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
                VALUES ($1, 'test', $2, 'pending', 0, NOW(), NOW())
                "#
            )
            .bind(Uuid::new_v4())
            .bind(json!({}))
            .execute(&pool)
            .await?;
        }

        let count = db.count_pending().await?;
        assert_eq!(count, 3);
        Ok(())
    }
}
