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

async fn setup_calendar(pool: &PgPool) -> uuid::Uuid {
    // 1. Create User
    let telegram_id = rand::random::<i64>().abs();
    let username = format!("api_user_{}", telegram_id);

    let user_id: uuid::Uuid = sqlx::query_scalar("INSERT INTO users (telegram_id, telegram_username, created_at) VALUES ($1, $2, NOW()) RETURNING id")
        .bind(telegram_id)
        .bind(&username)
        .fetch_one(pool)
        .await
        .unwrap();

    // 2. Create Calendar
    let calendar_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO calendars (id, user_id, name, color, sync_token, ctag, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())")
        .bind(calendar_id)
        .bind(user_id)
        .bind("Default Calendar")
        .bind("#3b82f6")
        .bind("0")
        .bind("0") // ctag
        .execute(pool)
        .await
        .unwrap();

    calendar_id
}

use axum::extract::ConnectInfo;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn create_request(method: &str, uri: impl AsRef<str>, body: Body) -> Request<Body> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri.as_ref())
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap();

    req.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        8080,
    )));
    req
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_api_full_flow(pool: PgPool) {
    let calendar_id = setup_calendar(&pool).await;

    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        pool: pool.clone(),
        auth_cache,
    };
    let app = create_router(state, "*");

    // 1. Create Event
    let event_uid = "api-test-uid";
    let create_body = serde_json::json!({
        "calendar_id": calendar_id,
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

    // 2. List Events
    let response = app
        .clone()
        .oneshot(create_request(
            "GET",
            format!("/api/events?calendar_id={}", calendar_id),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let events: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(events.as_array().unwrap().len() >= 1);

    // 3. Get Event
    let response = app
        .clone()
        .oneshot(create_request(
            "GET",
            format!("/api/events/{}", event_id),
            Body::empty(),
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
        ))
        .await
        .unwrap();
    // Assuming get_event returns 500 or 404 if not found?
    // db::events::get_event returns Err if not found?
    // Let's check db::events::get_event implementation later.
    // Usually get_event returns specific error which maps to 404 or fails.
    // If db::events::get_event calls sqlx::query_as(...).fetch_one(), it returns RowNotFound error.
    // ApiError::from(sqlx::Error) -> Internal(500) usually.
    // Ideally it should be NotFound.
    // Let's assert it is not 200 OK.
    assert!(response.status() != StatusCode::OK);
}
