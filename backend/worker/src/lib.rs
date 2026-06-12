//! Televent Worker - Background job processor
//!
//! Processes typed outbox messages with retry logic

#[cfg(test)]
mod bench_worker;
mod config;
mod db;
mod processors;

pub use config::Config;
pub use db::{WorkerDb, WorkerDbError};

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use televent_application::{CalendarService, EventView};
use televent_domain::OutboxPayload;
use teloxide::Bot;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Run the background worker service
///
/// This function runs the job processing loop until cancelled or an error occurs.
///
/// # Arguments
/// * `db` - Worker outbox repository
/// * `calendar` - Calendar application service
/// * `bot` - Telegram bot instance for sending notifications
/// * `config` - Worker configuration
/// * `shutdown` - Optional cancellation token for graceful shutdown
pub async fn run_worker(
    db: WorkerDb,
    calendar: CalendarService,
    bot: Bot,
    config: Config,
    shutdown: Option<CancellationToken>,
) -> Result<()> {
    info!(
        "Starting worker: poll_interval={}s, max_retries={}, batch_size={}",
        config.poll_interval_secs, config.max_retry_count, config.batch_size
    );

    run_worker_loop(db, calendar, bot, config, shutdown).await
}

/// Main worker processing loop
async fn run_worker_loop(
    db: WorkerDb,
    calendar: CalendarService,
    bot: Bot,
    config: Config,
    shutdown: Option<CancellationToken>,
) -> Result<()> {
    let poll_interval = tokio::time::Duration::from_secs(config.poll_interval_secs);
    let mut last_status_log_time = Instant::now()
        .checked_sub(Duration::from_secs(config.status_log_interval_secs))
        .unwrap_or_else(Instant::now);

    loop {
        // Check for shutdown signal
        if let Some(ref token) = shutdown
            && token.is_cancelled()
        {
            info!("Worker received shutdown signal");
            break;
        }

        // Fetch pending jobs
        match db.fetch_pending_jobs(config.batch_size).await {
            Ok(jobs) if jobs.is_empty() => {
                // No jobs to process, sleep
                tokio::time::sleep(poll_interval).await;
                continue;
            }
            Ok(batch) => {
                let jobs = batch.jobs;
                let mut results = batch.failed_results;
                info!("Processing {} typed jobs concurrently", jobs.len());

                // Pre-fetch events for invite notifications to avoid N+1 queries
                let mut events_map = HashMap::new();
                let event_ids = invite_notification_event_ids(&jobs);

                if !event_ids.is_empty() {
                    match calendar.get_event_views_by_ids_any(&event_ids).await {
                        Ok(events) => {
                            for event in events {
                                events_map.insert(event.id, event);
                            }
                            info!(
                                "Pre-fetched {} events for batch processing",
                                events_map.len()
                            );
                        }
                        Err(e) => {
                            warn!("Failed to pre-fetch events: {}", e);
                            // We continue without cache, individual processors will fetch events (and fail/retry if DB is down)
                        }
                    }
                }

                let events_cache = Arc::new(events_map);

                // Process jobs concurrently. Keep each job id next to its JoinHandle so a
                // task panic/cancellation can still update the claimed outbox row.
                // This provides ~Nx throughput improvement where N = batch_size
                let mut tasks = Vec::with_capacity(jobs.len());

                for job in jobs {
                    let job_id = job.id;
                    let calendar = calendar.clone();
                    let bot = bot.clone();
                    let config = config.clone();
                    let events_cache = events_cache.clone();
                    let handle = tokio::spawn(async move {
                        process_job(&calendar, &bot, &config, job, events_cache).await
                    });
                    tasks.push((job_id, handle));
                }

                // Wait for all concurrent jobs to complete and collect results
                results.reserve(tasks.len());
                results.extend(collect_task_results(tasks).await);

                // Bulk update jobs
                if let Err(e) = db.bulk_update_jobs(results).await {
                    error!("Failed to bulk update jobs: {}", e);
                }

                // Log queue status
                if last_status_log_time.elapsed()
                    >= Duration::from_secs(config.status_log_interval_secs)
                {
                    if let Ok(pending_count) = db.count_pending().await
                        && pending_count > 0
                    {
                        info!("Queue status: {} pending jobs remaining", pending_count);
                    }
                    last_status_log_time = Instant::now();
                }
            }
            Err(e) => {
                error!("Failed to fetch pending jobs: {}", e);
                tokio::time::sleep(poll_interval).await;
            }
        }
    }

    Ok(())
}

/// Process a single job
pub(crate) async fn process_job(
    calendar: &CalendarService,
    bot: &Bot,
    config: &Config,
    job: db::TypedOutboxMessage,
    events_cache: Arc<HashMap<Uuid, EventView>>,
) -> db::JobResult {
    info!(
        "Processing job {} (type: {}, retry: {})",
        job.id,
        job.payload.kind().as_str(),
        job.retry_count
    );

    match processors::process_message(calendar, &job, bot, &events_cache).await {
        Ok(()) => {
            // Job succeeded
            info!("Job {} completed successfully", job.id);
            db::JobResult::Completed(job.id)
        }
        Err(e) => {
            // Job failed
            warn!("Job {} failed: {}", job.id, e);
            let error_msg = e.to_string();

            if job.retry_count < config.max_retry_count {
                // Retry with exponential backoff
                let backoff_minutes = 2_i64.pow((job.retry_count + 1) as u32);
                let next_scheduled = Utc::now() + ChronoDuration::minutes(backoff_minutes);
                info!(
                    "Rescheduling job {} for retry {} in {} minutes",
                    job.id,
                    job.retry_count + 1,
                    backoff_minutes
                );

                db::JobResult::Reschedule {
                    id: job.id,
                    retry_count: job.retry_count + 1,
                    scheduled_at: next_scheduled,
                    error: error_msg,
                }
            } else {
                // Max retries reached, mark as failed
                error!(
                    "Job {} exceeded max retries ({}), marking as failed",
                    job.id, config.max_retry_count
                );

                db::JobResult::Failed {
                    id: job.id,
                    error: error_msg,
                }
            }
        }
    }
}

fn invite_notification_event_ids(jobs: &[db::TypedOutboxMessage]) -> Vec<Uuid> {
    jobs.iter()
        .filter_map(|job| match &job.payload {
            OutboxPayload::InviteNotification(payload) => Some(payload.event_id),
            _ => None,
        })
        .collect()
}

async fn collect_task_results(
    tasks: Vec<(Uuid, tokio::task::JoinHandle<db::JobResult>)>,
) -> Vec<db::JobResult> {
    let mut results = Vec::with_capacity(tasks.len());

    for (job_id, task) in tasks {
        match task.await {
            Ok(result) => results.push(result),
            Err(err) => {
                error!("Worker task for job {} failed to join: {}", job_id, err);
                results.push(db::JobResult::Failed {
                    id: job_id,
                    error: format!("worker task failed: {err}"),
                });
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use televent_domain::InviteNotification;

    #[test]
    fn test_exponential_backoff() {
        // Test backoff calculation
        let retry_counts = [0, 1, 2, 3, 4];
        let expected_minutes = [2, 4, 8, 16, 32];

        for (retry, expected) in retry_counts.iter().zip(expected_minutes.iter()) {
            let backoff = 2_i64.pow((retry + 1) as u32);
            assert_eq!(backoff, *expected);
        }
    }

    #[test]
    fn test_config_structure() {
        // Verify Config can be constructed
        let cfg = Config {
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
        };

        assert_eq!(cfg.poll_interval_secs, 10);
        assert_eq!(cfg.max_retry_count, 5);
    }

    #[test]
    fn invite_notification_event_ids_uses_typed_payloads() {
        let event_id = Uuid::new_v4();
        let job = db::TypedOutboxMessage {
            id: Uuid::new_v4(),
            payload: OutboxPayload::InviteNotification(InviteNotification {
                event_id,
                target_user_id: 123,
            }),
            retry_count: 0,
        };

        assert_eq!(invite_notification_event_ids(&[job]), vec![event_id]);
    }

    #[tokio::test]
    async fn collect_task_results_marks_join_failures_failed() {
        let ok_id = Uuid::new_v4();
        let panic_id = Uuid::new_v4();
        let tasks = vec![
            (
                ok_id,
                tokio::spawn(async move { db::JobResult::Completed(ok_id) }),
            ),
            (
                panic_id,
                tokio::spawn(async move {
                    let _job_id = panic_id;
                    panic!("simulated worker panic");
                }),
            ),
        ];

        let results = collect_task_results(tasks).await;

        assert_eq!(results.len(), 2);
        assert!(matches!(results[0], db::JobResult::Completed(id) if id == ok_id));
        match &results[1] {
            db::JobResult::Failed { id, error } => {
                assert_eq!(*id, panic_id);
                assert!(error.contains("worker task failed"));
            }
            other => panic!("expected failed join result, got {other:?}"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_malformed_job_failed_without_retry(pool: PgPool) -> anyhow::Result<()> {
        use crate::db::OutboxStatus;
        use serde_json::json;

        let db = WorkerDb::new(pool.clone());

        let id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, kind, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'telegram_notification', $2, 'pending', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({}))
        .execute(&pool)
        .await?;

        let batch = db.fetch_pending_jobs(10).await?;
        assert!(batch.jobs.is_empty());
        db.bulk_update_jobs(batch.failed_results).await?;

        let (status, retry_count): (OutboxStatus, i32) =
            sqlx::query_as("SELECT status, retry_count FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(status, OutboxStatus::Failed);
        assert_eq!(retry_count, 0);
        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_malformed_job_failed_even_at_max_retries(pool: PgPool) -> anyhow::Result<()> {
        use crate::db::OutboxStatus;
        use serde_json::json;

        let db = WorkerDb::new(pool.clone());

        let id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, kind, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'telegram_notification', $2, 'pending', 5, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({}))
        .execute(&pool)
        .await?;

        let batch = db.fetch_pending_jobs(10).await?;
        assert!(batch.jobs.is_empty());
        db.bulk_update_jobs(batch.failed_results).await?;

        let (status, retry_count): (OutboxStatus, i32) =
            sqlx::query_as("SELECT status, retry_count FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(status, OutboxStatus::Failed);
        assert_eq!(retry_count, 5);
        Ok(())
    }
}
