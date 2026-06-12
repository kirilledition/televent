use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::StorageResult;

/// Outbox message row owned by the storage layer.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OutboxMessage {
    pub id: Uuid,
    pub kind: String,
    #[sqlx(json)]
    pub payload: serde_json::Value,
    pub status: OutboxStatus,
    pub retry_count: i32,
    pub scheduled_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

/// Outbox message status stored in Postgres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "outbox_status", rename_all = "lowercase")]
pub enum OutboxStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub enum OutboxUpdate {
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

#[derive(Clone)]
pub struct OutboxRepository {
    pool: PgPool,
}

impl OutboxRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn claim_pending_jobs(&self, batch_size: i64) -> StorageResult<Vec<OutboxMessage>> {
        claim_pending_jobs(&self.pool, batch_size).await
    }

    pub async fn count_pending(&self) -> StorageResult<i64> {
        count_pending(&self.pool).await
    }

    pub async fn mark_completed(&self, message_id: Uuid) -> StorageResult<()> {
        mark_completed(&self.pool, message_id).await
    }

    pub async fn mark_failed(&self, message_id: Uuid, error_msg: &str) -> StorageResult<()> {
        mark_failed(&self.pool, message_id, error_msg).await
    }

    pub async fn reschedule_message(
        &self,
        message_id: Uuid,
        current_retry_count: i32,
        error_msg: &str,
    ) -> StorageResult<()> {
        reschedule_message(&self.pool, message_id, current_retry_count, error_msg).await
    }

    pub async fn apply_updates(&self, updates: Vec<OutboxUpdate>) -> StorageResult<()> {
        apply_updates(&self.pool, updates).await
    }
}

async fn claim_pending_jobs(pool: &PgPool, batch_size: i64) -> StorageResult<Vec<OutboxMessage>> {
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
        RETURNING id, kind, payload, status, retry_count, scheduled_at, processed_at
        "#,
    )
    .bind(batch_size)
    .fetch_all(pool)
    .await?;

    Ok(messages)
}

async fn count_pending(pool: &PgPool) -> StorageResult<i64> {
    let result = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM outbox_messages
        WHERE status = 'pending'
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(result)
}

async fn mark_completed(pool: &PgPool, message_id: Uuid) -> StorageResult<()> {
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

async fn mark_failed(pool: &PgPool, message_id: Uuid, error_msg: &str) -> StorageResult<()> {
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
    .execute(pool)
    .await?;

    Ok(())
}

async fn reschedule_message(
    pool: &PgPool,
    message_id: Uuid,
    current_retry_count: i32,
    error_msg: &str,
) -> StorageResult<()> {
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
    .execute(pool)
    .await?;

    Ok(())
}

async fn apply_updates(pool: &PgPool, updates: Vec<OutboxUpdate>) -> StorageResult<()> {
    let mut completed_ids = Vec::new();

    let mut failed_ids = Vec::new();
    let mut failed_errors = Vec::new();

    let mut reschedule_ids = Vec::new();
    let mut reschedule_counts = Vec::new();
    let mut reschedule_times = Vec::new();
    let mut reschedule_errors = Vec::new();

    for update in updates {
        match update {
            OutboxUpdate::Completed(id) => completed_ids.push(id),
            OutboxUpdate::Failed { id, error } => {
                failed_ids.push(id);
                failed_errors.push(error);
            }
            OutboxUpdate::Reschedule {
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

    let mut tx = pool.begin().await?;

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
