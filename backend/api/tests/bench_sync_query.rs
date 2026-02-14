use api::{AppState, create_router};
use moka::future::Cache;
use sqlx::{PgPool, Row};
use std::time::{Duration, Instant};

#[sqlx::test]
#[ignore]
async fn bench_sync_query(pool: PgPool) {
    // Setup state
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        pool: pool.clone(),
        auth_cache,
        telegram_bot_token: "test_token".to_string(),
    };
    let _app = create_router(state, "*", ".");

    // Create user
    let user_id = 123456789i64;
    sqlx::query("INSERT INTO users (telegram_id, telegram_username, sync_token) VALUES ($1, 'bench_user', '0')")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to create user");

    // Insert 1000 events
    println!("Inserting 1000 events...");
    let mut tx = pool.begin().await.expect("Failed to begin transaction");

    for i in 0..1000 {
        let uid = format!("uid-{}", i);
        let summary = format!("Event {}", i);
        let version = (i % 100) + 1; // version 1 to 100

        sqlx::query(
            r#"
            INSERT INTO events (
                user_id, uid, summary, description, location,
                start, "end", is_all_day, status, timezone, rrule, etag, version,
                updated_at, created_at
            )
            VALUES ($1, $2, $3, NULL, NULL, NOW(), NOW() + INTERVAL '1 hour', FALSE, 'CONFIRMED', 'UTC', NULL, 'etag', $4, NOW(), NOW())
            "#
        )
        .bind(user_id)
        .bind(uid)
        .bind(summary)
        .bind(version)
        .execute(&mut *tx)
        .await
        .expect("Failed to insert event");
    }
    tx.commit().await.expect("Failed to commit transaction");
    println!("Inserted 1000 events.");

    // 1. Analyze Query Plan
    println!("--- EXPLAIN ANALYZE ---");
    // This query MUST match the one in crates/api/src/db/events.rs:list_events_since_sync
    // Current implementation uses ORDER BY version
    let query_str = r#"
        EXPLAIN ANALYZE SELECT * FROM events
        WHERE user_id = $1
        AND version > $2
        ORDER BY version ASC
    "#;

    // Use query_as not needed for EXPLAIN
    let rows = sqlx::query(query_str)
        .bind(user_id)
        .bind(0) // sync_token = 0
        .fetch_all(&pool)
        .await
        .expect("Failed to run EXPLAIN ANALYZE");

    println!("Query Plan (ORDER BY version):");
    for row in rows {
        let line: String = row.get(0);
        println!("{}", line);
    }
    println!("-----------------------");

    // 2. Benchmark DB Query execution
    println!("Benchmarking DB Query execution...");
    let start = Instant::now();

    // Replicating list_events_since_sync logic
    // We fetch into generic Row or discard result to measure time
    let _events = sqlx::query(
        r#"
        SELECT * FROM events
        WHERE user_id = $1
        AND version > $2
        ORDER BY version ASC
        "#,
    )
    .bind(user_id)
    .bind(0)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch events");

    let duration = start.elapsed();
    println!("DB Query Duration: {:?}", duration);
    println!("Fetched {} events", _events.len());
}
