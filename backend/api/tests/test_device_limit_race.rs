use api::AppState;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hmac::{Hmac, Mac};
use moka::future::Cache;
use sha2::Sha256;
use sqlx::PgPool;
use std::time::Duration;
use televent_core::models::UserId;
use tower::ServiceExt;
use urlencoding::encode;

// Helper to generate valid init data
fn generate_init_data(user_id: i64, bot_token: &str) -> String {
    let user_json = format!(r#"{{"id":{},"first_name":"Test","last_name":"User"}}"#, user_id);
    let auth_date = chrono::Utc::now().timestamp().to_string();

    let params = vec![
        ("auth_date", auth_date.as_str()),
        ("query_id", "AAGPK..."),
        ("user", user_json.as_str()),
    ];

    let mut keys: Vec<String> = params.iter().map(|(k, _)| k.to_string()).collect();
    keys.sort();

    let mut data_check_string = String::new();
    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            data_check_string.push('\n');
        }
        let val = params.iter().find(|(k, _)| *k == key).unwrap().1;
        data_check_string.push_str(key);
        data_check_string.push('=');
        data_check_string.push_str(val);
    }

    type HmacSha256 = Hmac<Sha256>;
    let secret_key = HmacSha256::new_from_slice(b"WebAppData")
        .unwrap()
        .chain_update(bot_token.as_bytes())
        .finalize()
        .into_bytes();

    let mut mac = HmacSha256::new_from_slice(&secret_key).unwrap();
    mac.update(data_check_string.as_bytes());
    let hash = hex::encode(mac.finalize().into_bytes());

    let mut query = String::new();
    for (k, v) in params {
        if !query.is_empty() {
            query.push('&');
        }
        query.push_str(k);
        query.push('=');
        query.push_str(&encode(v));
    }
    query.push_str("&hash=");
    query.push_str(&hash);
    query
}

#[sqlx::test(migrations = "../migrations")]
#[ignore] // Ignored by default as it requires a running DB and might be flaky due to timing
async fn test_device_limit_race_condition(pool: PgPool) {
    let user_id = UserId::new(123456789);

    // Create user
    sqlx::query("INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) VALUES ($1, 'test_user', 'UTC', '0', '0')")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();
    let token = "test_token";

    let state = AppState {
        pool: pool.clone(),
        auth_cache,
        telegram_bot_token: token.to_string(),
    };

    let app = api::create_router(state, "*");
    let init_data = generate_init_data(123456789, token);
    let auth_header = format!("tma {}", init_data);

    // Spawn 20 requests concurrently
    let mut tasks = tokio::task::JoinSet::new();

    for i in 0..20 {
        let app = app.clone();
        let header = auth_header.clone();
        let body = format!(r#"{{"name": "Device {}"}}"#, i);

        tasks.spawn(async move {
            let req = Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("Authorization", &header)
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap();

            app.oneshot(req).await.unwrap()
        });
    }

    // Wait for all requests
    let mut success_count = 0;
    while let Some(res) = tasks.join_next().await {
        let response = res.unwrap();
        if response.status() == StatusCode::CREATED {
            success_count += 1;
        }
    }

    // Verify count in DB
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM device_passwords WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    println!("Success count: {}, DB count: {}", success_count, count);

    // Assert limit enforced
    assert!(count <= 10, "Race condition allowed creating {} devices (limit 10)", count);
}
