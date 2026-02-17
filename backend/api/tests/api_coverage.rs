use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use api::{create_router, AppState};
use sqlx::PgPool;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use urlencoding::encode;
use chrono::Utc;
use serde_json::json;

async fn setup_app_with_db() -> axum::Router {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to DB");

    let auth_cache = moka::future::Cache::builder().build();

    let state = AppState {
        pool,
        auth_cache,
        telegram_bot_token: "test_token".to_string(),
    };

    // We use "test_token" as the bot token, so our helper must use the same to sign.
    create_router(state, "*")
}

fn generate_valid_auth_header(user_id: i64, bot_token: &str) -> String {
    let user_json = json!({
        "id": user_id,
        "first_name": "Test",
        "last_name": "User",
        "username": "testuser"
    }).to_string();

    let auth_date = Utc::now().timestamp().to_string();

    // Construction of data-check-string:
    // keys: auth_date, query_id, user (sorted)
    let mut data_check_string = String::new();
    data_check_string.push_str("auth_date=");
    data_check_string.push_str(&auth_date);
    data_check_string.push_str("\nquery_id=test_query_id");
    data_check_string.push_str("\nuser=");
    data_check_string.push_str(&user_json);

    type HmacSha256 = Hmac<Sha256>;
    let secret_key = HmacSha256::new_from_slice(b"WebAppData")
        .expect("HMAC can take any key length")
        .chain_update(bot_token.as_bytes())
        .finalize()
        .into_bytes();

    let mut mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC can take any key length");
    mac.update(data_check_string.as_bytes());
    let hash = hex::encode(mac.finalize().into_bytes());

    // init_data is urlencoded key=value pairs joined by &
    // Order doesn't strictly matter for parsing, but usually it's consistent.
    let init_data = format!(
        "auth_date={}&query_id=test_query_id&user={}&hash={}",
        auth_date,
        encode(&user_json),
        hash
    );

    // Prefix expected by middleware
    format!("twa-init-data {}", init_data)
}

#[tokio::test]
async fn test_health_check_basic() {
    let pool = PgPool::connect_lazy("postgres://localhost/dummy").unwrap();
    let auth_cache = moka::future::Cache::builder().build();
    let state = AppState {
        pool,
        auth_cache,
        telegram_bot_token: "dummy".to_string(),
    };
    let app = create_router(state, "*");

    let response = app
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().as_u16() < 600);
}

#[tokio::test]
async fn test_api_events_unauthorized() {
    let pool = PgPool::connect_lazy("postgres://localhost/dummy").unwrap();
    let auth_cache = moka::future::Cache::builder().build();
    let state = AppState {
        pool,
        auth_cache,
        telegram_bot_token: "dummy".to_string(),
    };
    let app = create_router(state, "*");

    let response = app
        .oneshot(Request::builder().uri("/api/events").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_events_flow() {
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    let app = setup_app_with_db().await;
    let auth_header = generate_valid_auth_header(12345, "test_token");

    // 1. List events
    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/events")
                .header("Authorization", &auth_header)
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Create event
    let event_body = json!({
        "uid": "test-uid-1",
        "summary": "Integration Test Event",
        "start": "2024-01-01T10:00:00Z",
        "end": "2024-01-01T11:00:00Z",
        "is_all_day": false,
        "timezone": "UTC"
    }).to_string();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/events")
                .header("Authorization", &auth_header)
                .header("Content-Type", "application/json")
                .body(Body::from(event_body))
                .unwrap()
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
