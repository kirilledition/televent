use api::{AppState, create_router};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use moka::future::Cache;
use sqlx::{PgPool};
use std::time::{Duration, Instant};
use televent_core::models::UserId;
use tower::ServiceExt;
use base64::{Engine, engine::general_purpose::STANDARD};
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};

#[sqlx::test(migrations = "../../migrations")]
async fn bench_caldav_put_attendees(pool: PgPool) {
    // 1. Setup Main User
    let user_id = UserId::new(1001);
    sqlx::query("INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) VALUES ($1, 'user_1001', 'UTC', '0', '0')")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let password = "password123";
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query("INSERT INTO device_passwords (id, user_id, password_hash, device_name) VALUES ($1, $2, $3, 'test_device')")
        .bind(uuid::Uuid::new_v4())
        .bind(user_id)
        .bind(password_hash)
        .execute(&pool)
        .await
        .unwrap();

    // 2. Setup 50 Attendee Users
    let num_attendees = 50;
    for i in 0..num_attendees {
        let attendee_id = UserId::new(2000 + i as i64);
        sqlx::query("INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) VALUES ($1, $2, 'UTC', '0', '0')")
            .bind(attendee_id)
            .bind(format!("user_{}", attendee_id))
            .execute(&pool)
            .await
            .unwrap();
    }

    // 3. Setup Router
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        pool: pool.clone(),
        auth_cache,
        telegram_bot_token: "test_token".to_string(),
    };
    let app = create_router(state, "*");

    // 4. Prepare ICAL body with 50 attendees
    let mut attendees_ical = String::new();
    for i in 0..num_attendees {
        let attendee_id = UserId::new(2000 + i as i64);
        // internal email format: user_<id>@televent.internal
        // Actually I should check what televent_core::attendee::parse_internal_email expects
        attendees_ical.push_str(&format!("ATTENDEE;CN=User {};RSVP=TRUE:mailto:user_{}@televent.internal\r\n", i, attendee_id));
    }

    let ical_body = format!(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//Test//Test//EN\r\n\
         BEGIN:VEVENT\r\n\
         UID:bench-event-123\r\n\
         DTSTAMP:20240101T000000Z\r\n\
         DTSTART:20240101T100000Z\r\n\
         DTEND:20240101T110000Z\r\n\
         SUMMARY:Bench Event\r\n\
         {}\
         END:VEVENT\r\n\
         END:VCALENDAR",
        attendees_ical
    );

    let credentials = format!("1001:{}", password);
    let encoded = STANDARD.encode(credentials.as_bytes());

    // 5. Run benchmark
    println!("Starting benchmark for CalDAV PUT with {} attendees...", num_attendees);
    let start = Instant::now();

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/caldav/1001/bench-event-123.ics"))
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

    let duration = start.elapsed();
    println!("CalDAV PUT with {} attendees took: {:?}", num_attendees, duration);

    assert_eq!(response.status(), StatusCode::CREATED);
}
