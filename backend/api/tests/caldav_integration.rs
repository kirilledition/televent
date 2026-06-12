use api::{AppState, create_router};
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use moka::future::Cache;
use sqlx::{PgPool, Row};
use std::time::Duration;
use televent_domain::{UserId, internal_email_for_telegram_id};
use tower::ServiceExt;

#[sqlx::test(migrations = "../migrations")]
async fn test_caldav_put_with_internal_attendee(pool: PgPool) {
    // 1. Setup Users
    let user_a_id = UserId::new(1001);
    let user_b_id = UserId::new(1002);

    sqlx::query("INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) VALUES ($1, 'user_a', 'UTC', 0, 0)")
        .bind(user_a_id.inner())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) VALUES ($1, 'user_b', 'UTC', 0, 0)")
        .bind(user_b_id.inner())
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
        .bind(user_a_id.inner())
        .bind(password_hash)
        .execute(&pool)
        .await
        .unwrap();

    // 3. Create Router
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        calendar_service: televent_application::CalendarService::new(
            televent_storage::calendar::CalendarRepository::new(pool.clone()),
        ),
        device_service: televent_application::DeviceService::new(
            televent_storage::device::DeviceRepository::new(pool.clone()),
        ),
        health_service: televent_application::HealthService::new(
            televent_storage::health::HealthRepository::new(pool.clone()),
        ),
        auth_cache,
        telegram_bot_token: "test_token".to_string(),
    };
    let app = create_router(state, "*");

    // 4. Create PUT Request with Internal Attendee
    let internal_email = internal_email_for_telegram_id(user_b_id.inner());
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
                .uri("/caldav/1001/test-event-123.ics")
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
    .bind(user_b_id.inner())
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(attendee.is_some(), "Attendee record should exist");
    let status: String = attendee.unwrap().get("status");
    assert_eq!(status, "NEEDS-ACTION"); // DB stores as string/enum

    // Check Outbox Notification
    let message =
        sqlx::query("SELECT payload FROM outbox_messages WHERE kind = 'invite_notification'")
            .fetch_optional(&pool)
            .await
            .unwrap();

    assert!(message.is_some(), "Outbox message should exist");
    let payload: serde_json::Value = message.unwrap().get("payload");

    assert_eq!(payload["target_user_id"], 1002);
    assert_eq!(payload["event_id"].as_str().unwrap(), event_id.to_string());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_caldav_put_replaces_attendees(pool: PgPool) {
    let user_a_id = UserId::new(1101);
    let user_b_id = UserId::new(1102);
    let user_c_id = UserId::new(1103);

    for (user_id, username) in [
        (user_a_id, "replace_user_a"),
        (user_b_id, "replace_user_b"),
        (user_c_id, "replace_user_c"),
    ] {
        sqlx::query(
            "INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) VALUES ($1, $2, 'UTC', 0, 0)",
        )
        .bind(user_id.inner())
        .bind(username)
        .execute(&pool)
        .await
        .unwrap();
    }

    let password = "password123";
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query("INSERT INTO device_passwords (id, user_id, password_hash, device_name) VALUES ($1, $2, $3, 'test_device')")
        .bind(uuid::Uuid::new_v4())
        .bind(user_a_id.inner())
        .bind(password_hash)
        .execute(&pool)
        .await
        .unwrap();

    let state = AppState {
        calendar_service: televent_application::CalendarService::new(
            televent_storage::calendar::CalendarRepository::new(pool.clone()),
        ),
        device_service: televent_application::DeviceService::new(
            televent_storage::device::DeviceRepository::new(pool.clone()),
        ),
        health_service: televent_application::HealthService::new(
            televent_storage::health::HealthRepository::new(pool.clone()),
        ),
        auth_cache: Cache::builder()
            .time_to_live(Duration::from_secs(300))
            .build(),
        telegram_bot_token: "test_token".to_string(),
    };
    let app = create_router(state, "*");
    let credentials = format!("{}:{}", user_a_id.inner(), password);
    let encoded = STANDARD.encode(credentials.as_bytes());

    let email_b = internal_email_for_telegram_id(user_b_id.inner());
    let email_c = internal_email_for_telegram_id(user_c_id.inner());
    let first_body = format!(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//Test//Test//EN\r\n\
         BEGIN:VEVENT\r\n\
         UID:replace-attendees-123\r\n\
         DTSTAMP:20240101T000000Z\r\n\
         DTSTART:20240101T100000Z\r\n\
         DTEND:20240101T110000Z\r\n\
         SUMMARY:Team Sync\r\n\
         ATTENDEE;CN=User B;RSVP=TRUE:mailto:{}\r\n\
         ATTENDEE;CN=User C;RSVP=TRUE:mailto:{}\r\n\
         END:VEVENT\r\n\
         END:VCALENDAR",
        email_b, email_c
    );

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/caldav/1101/replace-attendees-123.ics")
                .header("Authorization", format!("Basic {}", encoded))
                .header("Content-Type", "text/calendar")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::from(first_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_response.status(), StatusCode::CREATED);

    let second_body = format!(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//Test//Test//EN\r\n\
         BEGIN:VEVENT\r\n\
         UID:replace-attendees-123\r\n\
         DTSTAMP:20240101T000000Z\r\n\
         DTSTART:20240101T100000Z\r\n\
         DTEND:20240101T110000Z\r\n\
         SUMMARY:Team Sync Updated\r\n\
         ATTENDEE;CN=User B;RSVP=TRUE:mailto:{}\r\n\
         END:VEVENT\r\n\
         END:VCALENDAR",
        email_b
    );

    let second_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/caldav/1101/replace-attendees-123.ics")
                .header("Authorization", format!("Basic {}", encoded))
                .header("Content-Type", "text/calendar")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8081,
                ))))
                .body(Body::from(second_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_response.status(), StatusCode::NO_CONTENT);

    let event_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM events WHERE uid = 'replace-attendees-123'")
            .fetch_one(&pool)
            .await
            .unwrap();

    let attendee_emails: Vec<String> =
        sqlx::query_scalar("SELECT email FROM event_attendees WHERE event_id = $1 ORDER BY email")
            .bind(event_id)
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(attendee_emails, vec![email_b]);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_health_check(pool: PgPool) {
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        calendar_service: televent_application::CalendarService::new(
            televent_storage::calendar::CalendarRepository::new(pool.clone()),
        ),
        device_service: televent_application::DeviceService::new(
            televent_storage::device::DeviceRepository::new(pool.clone()),
        ),
        health_service: televent_application::HealthService::new(
            televent_storage::health::HealthRepository::new(pool.clone()),
        ),
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
