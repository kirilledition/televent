#[cfg(test)]
mod tests {
    use crate::{Config, WorkerDb, process_job};
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use televent_application::{CalendarService, UserId};
    use teloxide::Bot;
    use tokio::task::JoinSet;
    use uuid::Uuid;

    #[tokio::test]
    async fn bench_worker_invite_processing() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());

        // Skip if no DB available (implied by connection failure)
        let pool_res = PgPoolOptions::new()
            .max_connections(50)
            .connect(&database_url)
            .await;

        let pool = match pool_res {
            Ok(p) => p,
            Err(_) => {
                println!("Skipping benchmark: DB connection failed");
                return Ok(());
            }
        };

        println!("Running migrations...");
        sqlx::migrate!("../migrations").run(&pool).await?;
        println!("Migrations done.");

        let db = WorkerDb::new(pool.clone());
        let calendar = CalendarService::new(televent_storage::calendar::CalendarRepository::new(
            pool.clone(),
        ));
        let config = Config {
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 100,
            status_log_interval_secs: 60,
        };

        let bot = Bot::new("token");

        let run_id = Uuid::new_v4();

        let user_id = UserId::new(123456789);
        sqlx::query(
            "INSERT INTO users (telegram_id, timezone, sync_token, ctag, created_at, updated_at)
             VALUES ($1, 'UTC', 0, 0, NOW(), NOW())
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id.inner())
        .execute(&pool)
        .await?;

        let job_count = 100;
        let mut job_ids = Vec::with_capacity(job_count);
        let mut payloads = Vec::with_capacity(job_count);

        for i in 0..job_count {
            let event_id = Uuid::new_v4();
            let job_id = Uuid::new_v4();
            job_ids.push(job_id);

            payloads.push(json!({
                "event_id": event_id.to_string(),
                "target_user_id": 987654321,
                "run_id": run_id.to_string()
            }));

            sqlx::query(
                "INSERT INTO events (id, user_id, uid, summary, start, \"end\", status, timezone, version, etag, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, NOW(), NOW() + interval '1 hour', $5, 'UTC', 1, 'etag', NOW(), NOW())"
            )
            .bind(event_id)
            .bind(user_id.inner())
            .bind(format!("uid-{}-{}", run_id, i))
            .bind(format!("Event {}", i))
            .bind("CONFIRMED")
            .execute(&pool)
            .await?;
        }

        for (i, job_id) in job_ids.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO outbox_messages (id, kind, payload, status, retry_count, scheduled_at, created_at)
                VALUES ($1, 'invite_notification', $2, 'pending', 0, NOW(), NOW())
                "#
            )
            .bind(job_id)
            .bind(&payloads[i])
            .execute(&pool)
            .await?;
        }

        println!(
            "Inserted {} jobs and events. Starting processing...",
            job_count
        );

        let start_time = Instant::now();

        let mut processed_count = 0;
        loop {
            println!("Fetching jobs...");
            let batch = db.fetch_pending_jobs(config.batch_size).await?;
            println!(
                "Fetched {} typed jobs and {} decode failures",
                batch.jobs.len(),
                batch.failed_results.len()
            );

            if batch.is_empty() {
                break;
            }

            let jobs = batch.jobs;
            let mut results = batch.failed_results;

            // Pre-fetch logic
            let mut events_map = HashMap::new();
            let event_ids = crate::invite_notification_event_ids(&jobs);

            if !event_ids.is_empty() {
                let events = calendar.get_event_views_by_ids_any(&event_ids).await?;
                for event in events {
                    events_map.insert(event.id, event);
                }
            }
            let events_cache = Arc::new(events_map);

            let mut tasks = JoinSet::new();
            for job in jobs {
                let calendar = calendar.clone();
                let bot = bot.clone();
                let config = config.clone();
                let events_cache = events_cache.clone();
                tasks.spawn(async move {
                    process_job(&calendar, &bot, &config, job, events_cache).await
                });
            }

            results.reserve(tasks.len());
            while let Some(res) = tasks.join_next().await {
                if let Ok(job_result) = res {
                    results.push(job_result);
                }
            }

            let count = results.len();
            db.bulk_update_jobs(results).await?;
            processed_count += count;

            if processed_count >= job_count {
                break;
            }
        }

        let duration = start_time.elapsed();
        println!("Processed {} jobs in {:?}", processed_count, duration);

        sqlx::query("DELETE FROM outbox_messages WHERE payload->>'run_id' = $1")
            .bind(run_id.to_string())
            .execute(&pool)
            .await?;

        Ok(())
    }
}
