use api::{AppState, create_router};
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use moka::future::Cache;
use sqlx::{PgPool, Row};
use std::time::Duration;
use televent_core::attendee::generate_internal_email;
use televent_core::models::UserId;
use tower::ServiceExt;

#[sqlx::test(migrations = "../migrations")]
async fn test_caldav_put_with_internal_attendee(pool: PgPool) {
    // 1. Setup Users
    let user_a_id = UserId::new(1001);
    let user_b_id = UserId::new(1002);

    sqlx::query("INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) VALUES ($1, 'user_a', 'UTC', '0', '0')")
        .bind(user_a_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) VALUES ($1, 'user_b', 'UTC', '0', '0')")
        .bind(user_b_id)
        .execute(&pool)
        .await
        .unwrap();

    // 2. Setup Auth for User A
    let password = "password123";
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query("INSERT INTO device_passwords (id, user_id, password_hash, device_name) VALUES ($1, $2, $3, 'test_device')")
        .bind(uuid::Uuid::new_v4())
        .bind(user_a_id)
        .bind(password_hash)
        .execute(&pool)
        .await
        .unwrap();

    // 3. Create Router
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        pool: pool.clone(),
        auth_cache,
        telegram_bot_token: "test_token".to_string(),
    };
    let app = create_router(state, "*");

    // 4. Create PUT Request with Internal Attendee
    let internal_email = generate_internal_email(user_b_id);
    let ical_body = format!(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//Test//Test//EN\r\n\
         BEGIN:VEVENT\r\n\
         UID:test-event-123\r\n\
         DTSTAMP:20240101T000000Z\r\n\
         DTSTART:20240101T100000Z\r\n\
         DTEND:20240101T110000Z\r\n\
         SUMMARY:Team Sync\r\n\
         ATTENDEE;CN=User B;RSVP=TRUE:mailto:{}\r\n\
         END:VEVENT\r\n\
         END:VCALENDAR",
        internal_email
    );

    let credentials = format!("1001:{}", password);
    let encoded = STANDARD.encode(credentials.as_bytes());

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/caldav/1001/test-event-123.ics".to_string())
                .header("Authorization", format!("Basic {}", encoded))
                .header("Content-Type", "text/calendar")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::from(ical_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // 5. Verify Database
    // Check Event created
    let event = sqlx::query("SELECT id FROM events WHERE uid = 'test-event-123'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let event_id: uuid::Uuid = event.get("id");

    // Check Event Attendee
    let attendee = sqlx::query(
        "SELECT status::text AS status FROM event_attendees WHERE event_id = $1 AND user_id = $2",
    )
    .bind(event_id)
    .bind(user_b_id)
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(attendee.is_some(), "Attendee record should exist");
    let status: String = attendee.unwrap().get("status");
    assert_eq!(status, "NEEDS-ACTION"); // DB stores as string/enum

    // Check Outbox Notification
    let message = sqlx::query(
        "SELECT payload FROM outbox_messages WHERE message_type = 'invite_notification'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(message.is_some(), "Outbox message should exist");
    let payload: serde_json::Value = message.unwrap().get("payload");

    assert_eq!(payload["target_user_id"], 1002);
    assert_eq!(payload["event_id"].as_str().unwrap(), event_id.to_string());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_health_check(pool: PgPool) {
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        pool,
        auth_cache,
        telegram_bot_token: "test_token".to_string(),
    };
    let app = create_router(state, "*");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
