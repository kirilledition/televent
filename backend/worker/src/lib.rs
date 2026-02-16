//! Televent Worker - Background job processor
//!
//! Processes outbox messages (emails, Telegram notifications) with retry logic

mod bench_worker;
mod config;
mod db;
mod mailer;
mod processors;

pub use config::Config;
pub use mailer::Mailer;

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use db::WorkerDb;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use televent_core::models::Event;
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
/// * `pool` - Database connection pool
/// * `bot` - Telegram bot instance for sending notifications
/// * `config` - Worker configuration
/// * `shutdown` - Optional cancellation token for graceful shutdown
pub async fn run_worker(
    pool: PgPool,
    bot: Bot,
    config: Config,
    shutdown: Option<CancellationToken>,
) -> Result<()> {
    let db = WorkerDb::new(pool.clone());
    let mailer = Mailer::new(&config)?;

    info!(
        "Starting worker: poll_interval={}s, max_retries={}, batch_size={}",
        config.poll_interval_secs, config.max_retry_count, config.batch_size
    );

    let pool_clone = pool.clone();
    run_worker_loop(pool_clone, db, bot, config, mailer, shutdown).await
}

/// Main worker processing loop
async fn run_worker_loop(
    pool: PgPool,
    db: WorkerDb,
    bot: Bot,
    config: Config,
    mailer: Mailer,
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
            Ok(jobs) => {
                info!("Processing {} jobs concurrently", jobs.len());

                // Pre-fetch events for invite notifications to avoid N+1 queries
                let mut events_map = HashMap::new();
                let event_ids: Vec<Uuid> = jobs
                    .iter()
                    .filter(|j| j.message_type == "invite_notification")
                    .filter_map(|j| {
                        j.payload
                            .get("event_id")
                            .and_then(|v| v.as_str())
                            .and_then(|s| Uuid::parse_str(s).ok())
                    })
                    .collect();

                if !event_ids.is_empty() {
                    // Fetch events in a single query
                    match sqlx::query_as::<_, Event>("SELECT * FROM events WHERE id = ANY($1)")
                        .bind(&event_ids)
                        .fetch_all(&pool)
                        .await
                    {
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

                // Process jobs concurrently using JoinSet
                // This provides ~Nx throughput improvement where N = batch_size
                let mut tasks = tokio::task::JoinSet::new();

                for job in jobs {
                    let pool = pool.clone();
                    let bot = bot.clone();
                    let config = config.clone();
                    let mailer = mailer.clone();
                    let events_cache = events_cache.clone();
                    tasks.spawn(async move {
                        process_job(&pool, &bot, &config, &mailer, job, events_cache).await
                    });
                }

                // Wait for all concurrent jobs to complete and collect results
                let mut results = Vec::with_capacity(tasks.len());
                while let Some(res) = tasks.join_next().await {
                    if let Ok(job_result) = res {
                        results.push(job_result);
                    }
                }

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
    pool: &PgPool,
    bot: &Bot,
    config: &Config,
    mailer: &Mailer,
    job: db::OutboxMessage,
    events_cache: Arc<HashMap<Uuid, Event>>,
) -> db::JobResult {
    info!(
        "Processing job {} (type: {}, retry: {})",
        job.id, job.message_type, job.retry_count
    );

    match processors::process_message(pool, &job, bot, mailer, &events_cache).await {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        use televent_core::config::CoreConfig;

        let cfg = Config {
            core: CoreConfig {
                database_url: "test".to_string(),
                telegram_bot_token: "test".to_string(),
                db_max_connections: 10,
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "test@example.com".to_string(),
            smtp_pool_size: 0,
        };

        assert_eq!(cfg.poll_interval_secs, 10);
        assert_eq!(cfg.max_retry_count, 5);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_process_job_email_success(pool: PgPool) -> sqlx::Result<()> {
        use serde_json::json;
        use televent_core::config::CoreConfig;
        use televent_core::models::OutboxStatus;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let db = WorkerDb::new(pool.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let config = Config {
            core: CoreConfig {
                database_url: "test".to_string(),
                telegram_bot_token: "test_token".to_string(),
                db_max_connections: 10,
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "127.0.0.1".to_string(),
            smtp_port: port,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "test@televent.app".to_string(),
            smtp_pool_size: 0,
        };

        let bot = Bot::new("token");
        let mailer = Mailer::new(&config).expect("Failed to create mailer");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(socket);
            let mut line = String::new();

            // Handshake
            reader
                .get_mut()
                .write_all(b"220 localhost ESMTP\r\n")
                .await
                .unwrap();

            loop {
                line.clear();
                let n = reader.read_line(&mut line).await.unwrap();
                if n == 0 {
                    break;
                }

                let cmd = line.split_whitespace().next().unwrap_or("").to_uppercase();
                match cmd.as_str() {
                    "EHLO" | "HELO" => {
                        reader
                            .get_mut()
                            .write_all(b"250-localhost\r\n250 8BITMIME\r\n")
                            .await
                            .unwrap();
                    }
                    "MAIL" => {
                        reader
                            .get_mut()
                            .write_all(b"250 2.1.0 Ok\r\n")
                            .await
                            .unwrap();
                    }
                    "RCPT" => {
                        reader
                            .get_mut()
                            .write_all(b"250 2.1.5 Ok\r\n")
                            .await
                            .unwrap();
                    }
                    "DATA" => {
                        reader
                            .get_mut()
                            .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                            .await
                            .unwrap();
                        loop {
                            line.clear();
                            let n = reader.read_line(&mut line).await.unwrap();
                            if n == 0 || line == ".\r\n" || line == ".\n" {
                                break;
                            }
                        }
                        reader
                            .get_mut()
                            .write_all(b"250 2.0.0 Ok: queued\r\n")
                            .await
                            .unwrap();
                    }
                    "QUIT" => {
                        reader
                            .get_mut()
                            .write_all(b"221 2.0.0 Bye\r\n")
                            .await
                            .unwrap();
                        break;
                    }
                    _ => {
                        reader
                            .get_mut()
                            .write_all(b"500 Command not recognized\r\n")
                            .await
                            .unwrap();
                    }
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'email', $2, 'processing', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({
            "to": "test@example.com",
            "subject": "Test",
            "body": "Body"
        }))
        .execute(&pool)
        .await?;

        let job =
            sqlx::query_as::<_, db::OutboxMessage>("SELECT * FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        let events_cache = Arc::new(HashMap::new());
        let result = process_job(&pool, &bot, &config, &mailer, job, events_cache).await;
        db.bulk_update_jobs(vec![result]).await?;

        let status: OutboxStatus =
            sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(status, OutboxStatus::Completed);
        server.await.unwrap();
        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_process_job_unknown_retry(pool: PgPool) -> sqlx::Result<()> {
        use serde_json::json;
        use televent_core::config::CoreConfig;
        use televent_core::models::OutboxStatus;

        let db = WorkerDb::new(pool.clone());
        let config = Config {
            core: CoreConfig {
                database_url: "test".to_string(),
                telegram_bot_token: "test_token".to_string(),
                db_max_connections: 10,
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "127.0.0.1".to_string(),
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "test@televent.app".to_string(),
            smtp_pool_size: 0,
        };
        let bot = Bot::new("token");
        let mailer = Mailer::new(&config).expect("Failed to create mailer");

        let id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'unknown_type', $2, 'processing', 0, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({}))
        .execute(&pool)
        .await?;

        let job =
            sqlx::query_as::<_, db::OutboxMessage>("SELECT * FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        let events_cache = Arc::new(HashMap::new());
        let result = process_job(&pool, &bot, &config, &mailer, job, events_cache).await;
        db.bulk_update_jobs(vec![result]).await?;

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
    async fn test_process_job_max_retries(pool: PgPool) -> sqlx::Result<()> {
        use serde_json::json;
        use televent_core::config::CoreConfig;
        use televent_core::models::OutboxStatus;

        let db = WorkerDb::new(pool.clone());
        let config = Config {
            core: CoreConfig {
                database_url: "test".to_string(),
                telegram_bot_token: "test_token".to_string(),
                db_max_connections: 10,
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "127.0.0.1".to_string(),
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "test@televent.app".to_string(),
            smtp_pool_size: 0,
        };
        let bot = Bot::new("token");
        let mailer = Mailer::new(&config).expect("Failed to create mailer");

        let id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
            VALUES ($1, 'unknown_type', $2, 'processing', 5, NOW(), NOW())
            "#
        )
        .bind(id)
        .bind(json!({}))
        .execute(&pool)
        .await?;

        let job =
            sqlx::query_as::<_, db::OutboxMessage>("SELECT * FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        let events_cache = Arc::new(HashMap::new());
        let result = process_job(&pool, &bot, &config, &mailer, job, events_cache).await;
        db.bulk_update_jobs(vec![result]).await?;

        let status: OutboxStatus =
            sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(status, OutboxStatus::Failed);
        Ok(())
    }
}
