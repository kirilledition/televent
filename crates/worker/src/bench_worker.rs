#[cfg(test)]
mod tests {
    use crate::{Config, Mailer, WorkerDb, process_job};
    use sqlx::PgPool;
    use std::time::Instant;
    use televent_core::config::CoreConfig;
    use teloxide::Bot;
    use tokio::task::JoinSet;
    use uuid::Uuid;

    #[sqlx::test(migrations = "../../migrations")]
    #[ignore]
    async fn bench_worker_processing(pool: PgPool) -> sqlx::Result<()> {
        let db = WorkerDb::new(pool.clone());
        let config = Config {
            core: CoreConfig {
                database_url: "test".to_string(),
                telegram_bot_token: "test_token".to_string(),
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 100, // Use a larger batch size for benchmark
            status_log_interval_secs: 60,
            smtp_host: "127.0.0.1".to_string(),
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "test@televent.app".to_string(),
        };
        // Use a dummy token; we won't actually send requests if we mock or if the token is invalid.
        // But process_job calls `processors::process_message`.
        // If message type is 'email', it tries to send email. If 'telegram_notification', it tries to send telegram.
        // I should use a message type that does minimal work or is easy to mock.
        // 'unknown_type' just fails and retries.
        // I want to test successful completion to hit `mark_completed`.
        // So I should use a type that succeeds.
        // `test_process_job_email_success` spins up a TCP listener. That's too heavy for 1000 jobs.
        // Maybe I can rely on a non-existent processor returning Ok?
        // `process_message` matches on message type.

        // Let's look at `processors.rs` to see if there is a no-op type or if I can add one.
        // If not, I'll use 'unknown_type' which fails, hitting `mark_failed` or `reschedule_message`.
        // `mark_failed` is also a DB update, so N+1 applies there too.
        // Let's use 'unknown_type' and set retry_count to max so it calls `mark_failed`.

        let bot = Bot::new("token");
        let mailer = Mailer::new(&config).expect("Failed to create mailer");

        // Insert 500 jobs
        let job_count = 500;
        let mut job_ids = Vec::with_capacity(job_count);

        for _ in 0..job_count {
            let id = Uuid::new_v4();
            job_ids.push(id);
        }

        // Bulk insert to speed up setup
        // sqlx doesn't support bulk insert easily without query builder, but we can use UNNEST
        sqlx::query(
            r#"
            INSERT INTO outbox_messages (id, message_type, payload, status, retry_count, scheduled_at, created_at)
            SELECT id, 'unknown_type', '{}'::jsonb, 'pending', $2, NOW(), NOW()
            FROM UNNEST($1::uuid[]) AS t(id)
            "#
        )
        .bind(&job_ids)
        .bind(config.max_retry_count) // Force fail immediately to skip retries
        .execute(&pool)
        .await?;

        println!("Inserted {} jobs. Starting processing...", job_count);

        // Fetch jobs
        // In real worker, this is done in a loop. Here we just fetch all of them (or in batches).
        // `fetch_pending_jobs` has a limit.
        // We will simulate the loop.

        let start_time = Instant::now();
        let mut processed_count = 0;

        loop {
            let jobs = db.fetch_pending_jobs(config.batch_size).await?;
            if jobs.is_empty() {
                break;
            }

            let mut tasks = JoinSet::new();
            for job in jobs {
                let bot = bot.clone();
                let config = config.clone();
                let mailer = mailer.clone();
                tasks.spawn(async move { process_job(&bot, &config, &mailer, job).await });
            }

            let mut results = Vec::new();
            while let Some(res) = tasks.join_next().await {
                if let Ok(job_result) = res {
                    results.push(job_result);
                }
            }

            let count = results.len();
            db.bulk_update_jobs(results).await?;
            processed_count += count;
        }

        let duration = start_time.elapsed();
        println!("Processed {} jobs in {:?}", job_count, duration);

        Ok(())
    }
}
