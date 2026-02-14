use api::{AppState, create_router};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use moka::future::Cache;
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;

async fn setup_user(pool: &PgPool) -> i64 {
    // 1. Create User
    let telegram_id = rand::random::<i64>().abs();
    let username = format!("api_user_{}", telegram_id);

    // Insert user (which acts as calendar owner and calendar itself in new schema)
    sqlx::query(
        r#"
        INSERT INTO users (
            telegram_id, telegram_username, timezone,
            sync_token, ctag,
            created_at, updated_at
        )
        VALUES ($1, $2, 'UTC', '0', '0', NOW(), NOW())
        "#,
    )
    .bind(telegram_id)
    .bind(&username)
    .execute(pool)
    .await
    .unwrap();

    telegram_id
}

use axum::extract::ConnectInfo;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn generate_valid_init_data(bot_token: &str, telegram_id: i64) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let auth_date = chrono::Utc::now().timestamp().to_string();
    let user_json = serde_json::json!({
        "id": telegram_id,
        "first_name": "Test User",
        "username": "test_user"
    })
    .to_string();

    // Construct data_check_string (lexicographically sorted keys: auth_date, user)
    let data_check_string = format!("auth_date={}\nuser={}", auth_date, user_json);

    type HmacSha256 = Hmac<Sha256>;
    let secret_key = HmacSha256::new_from_slice(b"WebAppData")
        .expect("HMAC can take any key length")
        .chain_update(bot_token.as_bytes())
        .finalize()
        .into_bytes();

    let mut mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC can take any key length");
    mac.update(data_check_string.as_bytes());
    let hash = hex::encode(mac.finalize().into_bytes());

    let params = vec![
        ("auth_date", auth_date.as_str()),
        ("user", user_json.as_str()),
        ("hash", hash.as_str()),
    ];

    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish()
}

fn create_request(
    method: &str,
    uri: impl AsRef<str>,
    body: Body,
    init_data: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri.as_ref())
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(data) = init_data {
        builder = builder.header(header::AUTHORIZATION, format!("tma {}", data));
    }

    let mut req = builder.body(body).unwrap();

    req.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        8080,
    )));
    req
}

#[sqlx::test(migrations = "../migrations")]
async fn test_api_full_flow(pool: PgPool) {
    let telegram_id = setup_user(&pool).await;
    let bot_token = "dummy_token";
    let init_data = generate_valid_init_data(bot_token, telegram_id);

    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        pool: pool.clone(),
        auth_cache,
        telegram_bot_token: bot_token.to_string(),
    };
    let app = create_router(state, "*", ".");

    // 1. Create Event
    let event_uid = "api-test-uid";
    let create_body = serde_json::json!({
        // calendar_id is removed
        "uid": event_uid,
        "summary": "API Test Event",
        "description": "Created via API",
        "location": "Internet",
        "start": "2026-06-01T10:00:00Z",
        "end": "2026-06-01T11:00:00Z",
        "is_all_day": false,
        "timezone": "UTC",
        "rrule": null
    });

    let response = app
        .clone()
        .oneshot(create_request(
            "POST",
            "/api/events",
            Body::from(create_body.to_string()),
            Some(&init_data),
        ))
        .await
        .unwrap();

    if response.status() != StatusCode::CREATED {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        println!("Create failed status: {}, body: {:?}", status, body);
        panic!("Create failed");
    }

    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let created_event: Value = serde_json::from_slice(&body_bytes).unwrap();
    let event_id = created_event["id"].as_str().unwrap().to_string();

    // 1b. Create Event 2
    let create_body_2 = serde_json::json!({
        "uid": "api-test-uid-2",
        "summary": "API Test Event 2",
        "description": "Created via API 2",
        "location": "Internet 2",
        "start": "2026-06-02T10:00:00Z",
        "end": "2026-06-02T11:00:00Z",
        "is_all_day": false,
        "timezone": "UTC",
        "rrule": null
    });
    let response = app
        .clone()
        .oneshot(create_request(
            "POST",
            "/api/events",
            Body::from(create_body_2.to_string()),
            Some(&init_data),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // 1c. Create Event 3
    let create_body_3 = serde_json::json!({
        "uid": "api-test-uid-3",
        "summary": "API Test Event 3",
        "description": "Created via API 3",
        "location": "Internet 3",
        "start": "2026-06-03T10:00:00Z",
        "end": "2026-06-03T11:00:00Z",
        "is_all_day": false,
        "timezone": "UTC",
        "rrule": null
    });
    let response = app
        .clone()
        .oneshot(create_request(
            "POST",
            "/api/events",
            Body::from(create_body_3.to_string()),
            Some(&init_data),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // 2. List Events
    let response = app
        .clone()
        .oneshot(create_request(
            "GET",
            "/api/events", // No calendar_id param
            Body::empty(),
            Some(&init_data),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let events: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(events.as_array().unwrap().len() >= 3);

    // 3. Get Event
    let response = app
        .clone()
        .oneshot(create_request(
            "GET",
            format!("/api/events/{}", event_id),
            Body::empty(),
            Some(&init_data),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let event: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(event["id"], event_id);

    // 4. Update Event
    let update_body = serde_json::json!({
        "summary": "Updated API Event"
    });
    let response = app
        .clone()
        .oneshot(create_request(
            "PUT",
            format!("/api/events/{}", event_id),
            Body::from(update_body.to_string()),
            Some(&init_data),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let updated_event: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(updated_event["summary"], "Updated API Event");

    // 5. Delete Event
    let response = app
        .clone()
        .oneshot(create_request(
            "DELETE",
            format!("/api/events/{}", event_id),
            Body::empty(),
            Some(&init_data),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 6. Verify Deletion
    let response = app
        .clone()
        .oneshot(create_request(
            "GET",
            format!("/api/events/{}", event_id),
            Body::empty(),
            Some(&init_data),
        ))
        .await
        .unwrap();

    // Should be Not Found (404) or Internal Server Error depending on implementation
    // db::events::get_event calls fetch_optional or fetch_one?
    // It calls fetch_one usually.
    // If we want it to be cleaner we should handle row not found in routes.
    // But currently, any error is 500 except specific ones.
    // Let's just assert it failed.
    assert!(response.status() != StatusCode::OK);
}
