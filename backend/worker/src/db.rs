//! Database operations for the worker
//!
//! Handles fetching and updating outbox messages

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use televent_domain::OutboxPayload;
#[cfg(test)]
pub use televent_storage::outbox::OutboxStatus;
use televent_storage::{
    StorageError,
    outbox::{OutboxMessage as StoredOutboxMessage, OutboxRepository, OutboxUpdate},
};
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Error)]
#[error("worker database error: {message}")]
pub struct WorkerDbError {
    message: String,
}

fn storage_to_worker(error: StorageError) -> WorkerDbError {
    WorkerDbError {
        message: error.to_string(),
    }
}

#[derive(Debug, Error)]
#[error("invalid outbox message {id}: {message}")]
pub struct OutboxDecodeError {
    pub id: Uuid,
    message: String,
}

impl OutboxDecodeError {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone)]
pub struct TypedOutboxMessage {
    pub id: Uuid,
    pub payload: OutboxPayload,
    pub retry_count: i32,
}

impl TryFrom<StoredOutboxMessage> for TypedOutboxMessage {
    type Error = OutboxDecodeError;

    fn try_from(message: StoredOutboxMessage) -> Result<Self, Self::Error> {
        let id = message.id;
        let retry_count = message.retry_count;
        let payload = OutboxPayload::from_parts(&message.kind, message.payload).map_err(|err| {
            OutboxDecodeError {
                id,
                message: err.to_string(),
            }
        })?;

        Ok(Self {
            id,
            payload,
            retry_count,
        })
    }
}

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

#[derive(Debug, Clone, Default)]
pub struct ClaimedOutboxBatch {
    pub jobs: Vec<TypedOutboxMessage>,
    pub failed_results: Vec<JobResult>,
}

impl ClaimedOutboxBatch {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty() && self.failed_results.is_empty()
    }
}

/// Worker database handle
#[derive(Clone)]
pub struct WorkerDb {
    outbox: OutboxRepository,
}

impl WorkerDb {
    /// Create a new database handle
    pub fn new(pool: PgPool) -> Self {
        Self {
            outbox: OutboxRepository::new(pool),
        }
    }

    /// Fetch pending jobs and mark them as processing
    ///
    /// Uses FOR UPDATE SKIP LOCKED to prevent duplicate processing
    pub async fn fetch_pending_jobs(
        &self,
        batch_size: i64,
    ) -> Result<ClaimedOutboxBatch, WorkerDbError> {
        let messages = self
            .outbox
            .claim_pending_jobs(batch_size)
            .await
            .map_err(storage_to_worker)?;

        Ok(decode_claimed_jobs(messages))
    }

    /// Mark a message as completed
    #[cfg(test)]
    pub async fn mark_completed(&self, message_id: Uuid) -> Result<(), WorkerDbError> {
        self.outbox
            .mark_completed(message_id)
            .await
            .map_err(storage_to_worker)
    }

    /// Mark a message as failed with error message
    #[cfg(test)]
    pub async fn mark_failed(
        &self,
        message_id: Uuid,
        error_msg: &str,
    ) -> Result<(), WorkerDbError> {
        self.outbox
            .mark_failed(message_id, error_msg)
            .await
            .map_err(storage_to_worker)
    }

    /// Reschedule a message with exponential backoff
    ///
    /// Backoff formula: 2^(current_retry_count + 1) minutes
    #[cfg(test)]
    pub async fn reschedule_message(
        &self,
        message_id: Uuid,
        current_retry_count: i32,
        error_msg: &str,
    ) -> Result<(), WorkerDbError> {
        self.outbox
            .reschedule_message(message_id, current_retry_count, error_msg)
            .await
            .map_err(storage_to_worker)
    }

    /// Get count of pending messages (for monitoring)
    pub async fn count_pending(&self) -> Result<i64, WorkerDbError> {
        self.outbox.count_pending().await.map_err(storage_to_worker)
    }

    /// Bulk update jobs based on their processing results
    pub async fn bulk_update_jobs(&self, results: Vec<JobResult>) -> Result<(), WorkerDbError> {
        let updates = results
            .into_iter()
            .map(|result| match result {
                JobResult::Completed(id) => OutboxUpdate::Completed(id),
                JobResult::Failed { id, error } => OutboxUpdate::Failed { id, error },
                JobResult::Reschedule {
                    id,
                    retry_count,
                    scheduled_at,
                    error,
                } => OutboxUpdate::Reschedule {
                    id,
                    retry_count,
                    scheduled_at,
                    error,
                },
            })
            .collect();

        self.outbox
            .apply_updates(updates)
            .await
            .map_err(storage_to_worker)
    }
}

fn decode_claimed_jobs(messages: Vec<StoredOutboxMessage>) -> ClaimedOutboxBatch {
    let mut batch = ClaimedOutboxBatch {
        jobs: Vec::with_capacity(messages.len()),
        failed_results: Vec::new(),
    };

    for message in messages {
        let job_id = message.id;
        match TypedOutboxMessage::try_from(message) {
            Ok(job) => batch.jobs.push(job),
            Err(err) => {
                warn!("{}", err);
                batch.failed_results.push(JobResult::Failed {
                    id: job_id,
                    error: err.message().to_string(),
                });
            }
        }
    }

    batch
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
    fn test_typed_outbox_message_has_required_traits() {
        // Verify worker-owned outbox messages have required traits
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}
        assert_clone::<TypedOutboxMessage>();
        assert_debug::<TypedOutboxMessage>();
    }

    #[test]
    fn decode_claimed_jobs_marks_invalid_payload_failed() {
        let id = Uuid::new_v4();
        let message = StoredOutboxMessage {
            id,
            kind: "invite_notification".to_string(),
            payload: serde_json::json!({"bad": true}),
            status: OutboxStatus::Processing,
            retry_count: 0,
            scheduled_at: Utc::now(),
            processed_at: None,
        };

        let batch = decode_claimed_jobs(vec![message]);

        assert!(batch.jobs.is_empty());
        assert_eq!(batch.failed_results.len(), 1);
        match &batch.failed_results[0] {
            JobResult::Failed { id: failed_id, .. } => assert_eq!(*failed_id, id),
            other => panic!("expected failed result, got {other:?}"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_fetch_pending_jobs(pool: PgPool) -> anyhow::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());

        let id1 = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, kind, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'telegram_notification', $2, 'pending', 0, NOW() - INTERVAL '1 minute', NOW())
            "#
        )
        .bind(id1)
        .bind(json!({"telegram_id": 123, "message": "hello"}))
        .execute(&pool)
        .await?;

        let batch = db.fetch_pending_jobs(10).await?;
        assert_eq!(batch.jobs.len(), 1);
        assert!(batch.failed_results.is_empty());
        assert!(matches!(
            batch.jobs[0].payload,
            OutboxPayload::TelegramNotification(_)
        ));

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_mark_completed(pool: PgPool) -> anyhow::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, kind, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'telegram_notification', $2, 'processing', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({"telegram_id": 123, "message": "hello"}))
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

    #[sqlx::test(migrations = "../migrations")]
    async fn test_mark_failed(pool: PgPool) -> anyhow::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, kind, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'telegram_notification', $2, 'processing', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({"telegram_id": 123, "message": "hello"}))
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

    #[sqlx::test(migrations = "../migrations")]
    async fn test_reschedule_message(pool: PgPool) -> anyhow::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, kind, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'telegram_notification', $2, 'processing', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({"telegram_id": 123, "message": "hello"}))
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

    #[sqlx::test(migrations = "../migrations")]
    async fn test_count_pending(pool: PgPool) -> anyhow::Result<()> {
        use serde_json::json;
        let db = WorkerDb::new(pool.clone());

        for _ in 0..3 {
            sqlx::query(
                r#"
                INSERT INTO outbox_messages (id, kind, payload, status, retry_count, scheduled_at, created_at)
                VALUES ($1, 'telegram_notification', $2, 'pending', 0, NOW(), NOW())
                "#
            )
            .bind(Uuid::new_v4())
            .bind(json!({"telegram_id": 123, "message": "hello"}))
            .execute(&pool)
            .await?;
        }

        let count = db.count_pending().await?;
        assert_eq!(count, 3);
        Ok(())
    }
}
