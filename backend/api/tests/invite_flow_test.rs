use api::{AppState, create_router};
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use moka::future::Cache;
use sqlx::{PgPool, Row};
use std::time::Duration;
use televent_application::ConfirmRsvpCommand;
use televent_domain::{ParticipationStatus, UserId, internal_email_for_telegram_id};
use tower::ServiceExt;

/// Integration test for the complete invite flow
///
/// Steps:
/// 1. Seed DB with User A (Organizer) and User B (Attendee)
/// 2. Mock API call: PUT /calendar/event with ICS inviting User B
/// 3. Assert event_attendees has User B with 'NEEDS-ACTION'
/// 4. Assert outbox_messages has pending 'invite_notification'
/// 5. Simulate Worker: mark message as sent
/// 6. Simulate Bot Callback: User B clicks "ACCEPTED"
/// 7. Assert event_attendees status is 'ACCEPTED'
/// 8. Mock API call: GET /calendar/event as User A
/// 9. Assert ICS contains PARTSTAT=ACCEPTED for User B
#[sqlx::test(migrations = "../migrations")]
async fn test_invite_flow_end_to_end(pool: PgPool) {
    // Initialize tracing for debugging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // =============================================================================
    // Step 1: Seed Database with Users
    // =============================================================================
    let user_a_id = UserId::new(1001);
    let user_b_id = UserId::new(1002);

    // Create User A (Organizer)
    sqlx::query(
        "INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) 
         VALUES ($1, 'user_a', 'UTC', 0, 0)",
    )
    .bind(user_a_id.inner())
    .execute(&pool)
    .await
    .unwrap();

    // Create User B (Attendee)
    sqlx::query(
        "INSERT INTO users (telegram_id, telegram_username, timezone, sync_token, ctag) 
         VALUES ($1, 'user_b', 'UTC', 0, 0)",
    )
    .bind(user_b_id.inner())
    .execute(&pool)
    .await
    .unwrap();

    // Setup authentication for User A
    let password = "test_password_123";
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query(
        "INSERT INTO device_passwords (id, user_id, password_hash, device_name) 
         VALUES ($1, $2, $3, 'test_device')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user_a_id.inner())
    .bind(password_hash)
    .execute(&pool)
    .await
    .unwrap();

    // =============================================================================
    // Step 2: API PUT - Create event with internal attendee (User B)
    // =============================================================================

    // Create router
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let calendar_service = televent_application::CalendarService::new(
        televent_storage::calendar::CalendarRepository::new(pool.clone()),
    );
    let state = AppState {
        calendar_service: calendar_service.clone(),
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

    // Generate internal email for User B
    let internal_email = internal_email_for_telegram_id(user_b_id.inner());

    // Create ICS with attendee (using same format as existing ical_to_event_data tests)
    let ical_body = format!(
        "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Test//Test//EN\nBEGIN:VEVENT\nUID:test-event-invite-123\nDTSTAMP:20240101T000000Z\nDTSTART:20240101T100000Z\nDTEND:20240101T110000Z\nSUMMARY:Team Sync with B\nATTENDEE;CN=User B;RSVP=TRUE:mailto:{}\nEND:VEVENT\nEND:VCALENDAR",
        internal_email
    );

    // Prepare auth header
    let credentials = format!("1001:{}", password);
    let encoded = STANDARD.encode(credentials.as_bytes());

    // Execute PUT request
    let mut put_request = Request::builder()
        .method("PUT")
        .uri("/caldav/1001/test-event-invite-123.ics")
        .header("Authorization", format!("Basic {}", encoded))
        .header("Content-Type", "text/calendar")
        .body(Body::from(ical_body))
        .unwrap();

    // Add ConnectInfo for rate limiting
    put_request
        .extensions_mut()
        .insert(axum::extract::ConnectInfo(
            "127.0.0.1:12345".parse::<std::net::SocketAddr>().unwrap(),
        ));

    let response = app.clone().oneshot(put_request).await.unwrap();

    // Debug: Print response if not successful
    let status = response.status();
    if status != StatusCode::CREATED {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        eprintln!("PUT failed with status {}: {}", status, body_str);
    }

    assert_eq!(
        status,
        StatusCode::CREATED,
        "PUT request should create the event"
    );

    // =============================================================================
    // Step 3: Assert event_attendees has User B with NEEDS-ACTION
    // =============================================================================

    // Get event ID
    let event = sqlx::query("SELECT id FROM events WHERE uid = 'test-event-invite-123'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let event_id: uuid::Uuid = event.get("id");

    // Check attendee record
    let attendee = sqlx::query(
        "SELECT status::text AS status, email, user_id FROM event_attendees 
         WHERE event_id = $1 AND user_id = $2",
    )
    .bind(event_id)
    .bind(user_b_id.inner())
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(attendee.is_some(), "Attendee record should exist");

    let attendee_row = attendee.unwrap();
    let status: String = attendee_row.get("status");
    let email: String = attendee_row.get("email");
    let user_id: Option<i64> = attendee_row.get("user_id");

    assert_eq!(
        status, "NEEDS-ACTION",
        "Initial status should be NEEDS-ACTION"
    );
    assert_eq!(email, internal_email, "Email should match internal email");
    assert_eq!(user_id, Some(1002), "User ID should be set");

    // =============================================================================
    // Step 4: Assert outbox_messages has pending invite_notification
    // =============================================================================

    let message = sqlx::query(
        r#"
        SELECT id, payload, status::text AS status FROM outbox_messages
        WHERE kind = 'invite_notification'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(message.is_some(), "Outbox message should exist");

    let message_row = message.unwrap();
    let message_id: uuid::Uuid = message_row.get("id");
    let payload: serde_json::Value = message_row.get("payload");
    let message_status: String = message_row.get("status");

    assert_eq!(message_status, "pending", "Message should be pending");
    assert_eq!(
        payload["target_user_id"], 1002,
        "Payload should contain User B ID"
    );
    assert_eq!(
        payload["event_id"].as_str().unwrap(),
        event_id.to_string(),
        "Payload should contain event ID"
    );

    // =============================================================================
    // Step 5: Simulate Worker - Mark message as completed
    // =============================================================================

    sqlx::query("UPDATE outbox_messages SET status = 'completed'::outbox_status, processed_at = NOW() WHERE id = $1")
        .bind(message_id)
        .execute(&pool)
        .await
        .unwrap();

    // Verify message was marked completed
    let updated_message =
        sqlx::query("SELECT status::text AS status FROM outbox_messages WHERE id = $1")
            .bind(message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let updated_status: String = updated_message.get("status");
    assert_eq!(
        updated_status, "completed",
        "Worker should mark message as completed"
    );

    // =============================================================================
    // Step 6: Simulate Bot Callback - User B accepts invite
    // =============================================================================

    calendar_service
        .confirm_rsvp(ConfirmRsvpCommand {
            event_id,
            attendee_user_id: user_b_id,
            status: ParticipationStatus::Accepted,
            attendee_name: "User B".to_string(),
        })
        .await
        .unwrap();

    // =============================================================================
    // Step 7: Assert event_attendees status is ACCEPTED
    // =============================================================================

    let updated_attendee = sqlx::query(
        "SELECT status::text AS status FROM event_attendees 
         WHERE event_id = $1 AND user_id = $2",
    )
    .bind(event_id)
    .bind(user_b_id.inner())
    .fetch_one(&pool)
    .await
    .unwrap();

    let new_status: String = updated_attendee.get("status");
    assert_eq!(
        new_status, "ACCEPTED",
        "Status should be updated to ACCEPTED after bot callback"
    );

    // Verify event version was incremented
    let updated_event = sqlx::query("SELECT version FROM events WHERE id = $1")
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let version: i32 = updated_event.get("version");
    assert!(version > 1, "Event version should be incremented");

    let updated_organizer =
        sqlx::query("SELECT sync_token, ctag FROM users WHERE telegram_id = $1")
            .bind(user_a_id.inner())
            .fetch_one(&pool)
            .await
            .unwrap();
    let organizer_sync_token: i64 = updated_organizer.get("sync_token");
    let organizer_ctag: i64 = updated_organizer.get("ctag");
    assert!(
        organizer_sync_token > 0,
        "Organizer sync token should be incremented"
    );
    assert_eq!(
        organizer_ctag, organizer_sync_token,
        "Organizer ctag should track sync token"
    );

    let rsvp_message = sqlx::query(
        r#"
        SELECT payload, status::text AS status FROM outbox_messages
        WHERE kind = 'rsvp_notification'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(
        rsvp_message.is_some(),
        "RSVP notification outbox message should exist"
    );
    let rsvp_message_row = rsvp_message.unwrap();
    let rsvp_payload: serde_json::Value = rsvp_message_row.get("payload");
    let rsvp_status: String = rsvp_message_row.get("status");
    assert_eq!(
        rsvp_status, "pending",
        "RSVP notification should be pending"
    );
    assert_eq!(
        rsvp_payload["organizer_telegram_id"],
        user_a_id.inner(),
        "RSVP notification should target the organizer"
    );
    assert_eq!(
        rsvp_payload["rsvp_status"], "Accepted",
        "RSVP notification should record the accepted status"
    );

    // =============================================================================
    // Step 8: API GET - Retrieve event as User A
    // =============================================================================

    let mut get_request = Request::builder()
        .method("GET")
        .uri("/caldav/1001/test-event-invite-123.ics")
        .header("Authorization", format!("Basic {}", encoded))
        .body(Body::empty())
        .unwrap();

    // Add ConnectInfo for rate limiting
    get_request
        .extensions_mut()
        .insert(axum::extract::ConnectInfo(
            "127.0.0.1:12345".parse::<std::net::SocketAddr>().unwrap(),
        ));

    let get_response = app.oneshot(get_request).await.unwrap();

    assert_eq!(
        get_response.status(),
        StatusCode::OK,
        "GET request should succeed"
    );

    // =============================================================================
    // Step 9: Assert ICS contains PARTSTAT=ACCEPTED for User B
    // =============================================================================

    let body_bytes = axum::body::to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ical_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    // Debug: print ICS to see what we got
    tracing::info!("ICS Output:\n{}", ical_str);

    assert!(
        ical_str.contains("ATTENDEE"),
        "ICS should contain ATTENDEE property"
    );
    // Handle RFC 5545 line folding - remove newlines and spaces to find the email
    let ical_normalized = ical_str.replace("\r\n ", "").replace("\n ", "");
    assert!(
        ical_normalized.contains(&internal_email),
        "ICS should contain User B's internal email. Got: {}",
        ical_str
    );
    assert!(
        ical_str.contains("PARTSTAT=ACCEPTED"),
        "ICS should contain PARTSTAT=ACCEPTED for User B"
    );

    tracing::info!("✅ Invite flow integration test completed successfully");
}
