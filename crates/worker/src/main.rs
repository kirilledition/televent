//! Televent Worker - Background job processor
//!
//! Processes outbox messages (emails, Telegram notifications) with retry logic

mod config;
mod db;
mod processors;

use anyhow::Result;
use config::Config;
use db::WorkerDb;
use sqlx::PgPool;
use teloxide::Bot;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,worker=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Televent worker");

    // Load configuration
    let config = Config::from_env()?;
    info!(
        "Configuration loaded: poll_interval={}s, max_retries={}, batch_size={}",
        config.poll_interval_secs, config.max_retry_count, config.batch_size
    );

    // Create database connection pool
    let pool = PgPool::connect(&config.database_url).await?;
    info!("Database connection pool established");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    info!("Database migrations completed");

    // Create database handle
    let db = WorkerDb::new(pool);

    // Initialize Telegram bot
    let bot = Bot::new(&config.bot_token);
    info!("Telegram bot initialized");

    // Start processing loop
    info!("Starting job processing loop");
    run_worker_loop(db, bot, config).await?;

    Ok(())
}

/// Main worker processing loop
async fn run_worker_loop(db: WorkerDb, bot: Bot, config: Config) -> Result<()> {
    let poll_interval = tokio::time::Duration::from_secs(config.poll_interval_secs);

    loop {
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
                if let Ok(pending_count) = db.count_pending().await {
                    if pending_count > 0 {
                        info!("Queue status: {} pending jobs remaining", pending_count);
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch pending jobs: {}", e);
                tokio::time::sleep(poll_interval).await;
            }
        }
    }
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

            if job.retry_count < config.max_retry_count {
                // Retry with exponential backoff
                let backoff_minutes = 2_i64.pow((job.retry_count + 1) as u32);
                info!(
                    "Rescheduling job {} for retry {} in {} minutes",
                    job.id,
                    job.retry_count + 1,
                    backoff_minutes
                );

                if let Err(e) = db.reschedule_message(job.id, job.retry_count).await {
                    error!("Failed to reschedule job {}: {}", job.id, e);
                }
            } else {
                // Max retries reached, mark as failed
                error!(
                    "Job {} exceeded max retries ({}), marking as failed",
                    job.id, config.max_retry_count
                );

                if let Err(e) = db.mark_failed(job.id).await {
                    error!("Failed to mark job {} as failed: {}", job.id, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_compiles() {
        // Basic compilation test
        assert!(true);
    }

    #[test]
    fn test_exponential_backoff() {
        // Test backoff calculation
        let retry_counts = vec![0, 1, 2, 3, 4];
        let expected_minutes = vec![1, 2, 4, 8, 16];

        for (retry, expected) in retry_counts.iter().zip(expected_minutes.iter()) {
            let backoff = 2_i64.pow((retry + 1) as u32);
            assert_eq!(backoff, *expected);
        }
    }
}
