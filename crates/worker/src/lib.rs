//! Televent Worker - Background job processor
//!
//! Processes outbox messages (emails, Telegram notifications) with retry logic

mod config;
mod db;
mod mailer;
mod processors;

pub use config::Config;

use anyhow::Result;
use db::WorkerDb;
use sqlx::PgPool;
use teloxide::Bot;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

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
    let db = WorkerDb::new(pool);

    info!(
        "Starting worker: poll_interval={}s, max_retries={}, batch_size={}",
        config.poll_interval_secs, config.max_retry_count, config.batch_size
    );

    run_worker_loop(db, bot, config, shutdown).await
}

/// Main worker processing loop
async fn run_worker_loop(
    db: WorkerDb,
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
            && token.is_cancelled() {
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
                info!("Processing {} jobs", jobs.len());

                // Process each job
                for job in jobs {
                    process_job(&db, &bot, &config, job).await;
                }

                // Log queue status
                if last_status_log_time.elapsed() >= Duration::from_secs(config.status_log_interval_secs)
                {
                    if let Ok(pending_count) = db.count_pending().await
                        && pending_count > 0 {
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
async fn process_job(db: &WorkerDb, bot: &Bot, config: &Config, job: db::OutboxMessage) {
    info!(
        "Processing job {} (type: {}, retry: {})",
        job.id, job.message_type, job.retry_count
    );

    match processors::process_message(&job, bot).await {
        Ok(()) => {
            // Job succeeded
            info!("Job {} completed successfully", job.id);

            if let Err(e) = db.mark_completed(job.id).await {
                error!("Failed to mark job {} as completed: {}", job.id, e);
            }
        }
        Err(e) => {
            // Job failed
            warn!("Job {} failed: {}", job.id, e);
            let error_msg = e.to_string();

            if job.retry_count < config.max_retry_count {
                // Retry with exponential backoff
                let backoff_minutes = 2_i64.pow((job.retry_count + 1) as u32);
                info!(
                    "Rescheduling job {} for retry {} in {} minutes",
                    job.id,
                    job.retry_count + 1,
                    backoff_minutes
                );

                if let Err(e) = db
                    .reschedule_message(job.id, job.retry_count, &error_msg)
                    .await
                {
                    error!("Failed to reschedule job {}: {}", job.id, e);
                }
            } else {
                // Max retries reached, mark as failed
                error!(
                    "Job {} exceeded max retries ({}), marking as failed",
                    job.id, config.max_retry_count
                );

                if let Err(e) = db.mark_failed(job.id, &error_msg).await {
                    error!("Failed to mark job {} as failed: {}", job.id, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {


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
}
