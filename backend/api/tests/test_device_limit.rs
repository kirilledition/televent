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
use tower::ServiceExt; // for oneshot
use urlencoding::encode;

// Helper to generate valid init data (same as in telegram_auth.rs tests)
fn generate_init_data(user_id: i64, bot_token: &str) -> String {
    let user_json = format!(
        r#"{{"id":{},"first_name":"Test","last_name":"User"}}"#,
        user_id
    );
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

    // Construct query string
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
async fn test_device_limit_enforcement(pool: PgPool) {
    let user_id = UserId::new(123456789);

    // Create user in DB
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
        pool,
        auth_cache,
        telegram_bot_token: token.to_string(),
    };

    // Create router with app state
    let app = api::create_router(state, "*");

    // Generate auth header
    let init_data = generate_init_data(123456789, token);
    let auth_header = format!("tma {}", init_data);

    // Try to create 11 devices (limit is 10)
    for i in 0..11 {
        let req_body = format!(r#"{{"name": "Device {}"}}"#, i);

        // We must clone app for each request because oneshot consumes it
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/devices")
                    .header("Authorization", &auth_header)
                    .header("Content-Type", "application/json")
                    .body(Body::from(req_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        if i < 10 {
            assert_eq!(
                response.status(),
                StatusCode::CREATED,
                "Failed to create device {}",
                i
            );
        } else {
            // The 11th device creation (i=10) MUST fail with 400 Bad Request
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "Should fail on 11th device creation"
            );
        }
    }
}
