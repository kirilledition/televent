use api::{AppState, create_router};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use moka::future::Cache;
use sqlx::PgPool;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tower::ServiceExt;

async fn setup_user_and_auth(pool: &PgPool) -> (i64, String, String) {
    // 1. Create User
    let telegram_id = rand::random::<i64>().abs();
    let username = format!("user_{}", telegram_id);

    // Insert user
    sqlx::query(
        r#"
        INSERT INTO users (
            telegram_id, telegram_username, timezone, 
            calendar_name, calendar_color, sync_token, ctag, 
            created_at, updated_at
        ) 
        VALUES ($1, $2, 'UTC', 'Default Calendar', '#3b82f6', '0', '0', NOW(), NOW())
        "#,
    )
    .bind(telegram_id)
    .bind(&username)
    .execute(pool)
    .await
    .unwrap();

    // 2. Create Device Password
    let password = "test_password";
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query(
        r#"
        INSERT INTO device_passwords (
            id, user_id, password_hash, device_name, created_at
        ) 
        VALUES (gen_random_uuid(), $1, $2, 'test_device', NOW())
        "#,
    )
    .bind(telegram_id)
    .bind(password_hash)
    .execute(pool)
    .await
    .unwrap();

    // 3. Generate Auth Header
    let credentials = format!("{}:{}", telegram_id, password);
    let encoded = STANDARD.encode(credentials.as_bytes());
    let auth_header = format!("Basic {}", encoded);

    (telegram_id, username, auth_header)
}

fn create_request(
    method: &str,
    uri: impl AsRef<str>,
    auth_header: &str,
    headers: Vec<(&str, &str)>,
    body: Body,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri.as_ref())
        .header(header::AUTHORIZATION, auth_header);

    for (k, v) in headers {
        builder = builder.header(k, v);
    }

    let mut req = builder.body(body).unwrap();

    // Add ConnectInfo for rate limiting
    req.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        8080,
    )));

    req
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_caldav_full_flow(pool: PgPool) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("api=debug,info")
        .try_init();

    let (telegram_id, _username, auth_header) = setup_user_and_auth(&pool).await;

    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        pool: pool.clone(),
        auth_cache,
        telegram_bot_token: "dummy_token".to_string(),
    };
    let app = create_router(state, "*");

    // 0. OPTIONS (Thunderbird check)
    let response = app
        .clone()
        .oneshot(create_request(
            "OPTIONS",
            format!("/caldav/{}/", telegram_id),
            &auth_header,
            vec![],
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    println!("Headers: {:?}", headers);
    assert!(headers.contains_key("dav"));
    assert!(headers.contains_key("allow"));

    // 1. PROPFIND /caldav/{telegram_id}/ (Depth: 0)
    let response = app
        .clone()
        .oneshot(create_request(
            "PROPFIND",
            format!("/caldav/{}/", telegram_id),
            &auth_header,
            vec![("Depth", "0")],
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::MULTI_STATUS);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    // Validate required properties (Thunderbird)
    assert!(body_str.contains("displayname"));
    assert!(body_str.contains("resourcetype"));
    assert!(body_str.contains("supported-calendar-component-set"));
    assert!(body_str.contains("getctag"));

    // 1b. PROPFIND /caldav/{telegram_id}/ (Depth: 1)
    let response = app
        .clone()
        .oneshot(create_request(
            "PROPFIND",
            format!("/caldav/{}/", telegram_id),
            &auth_header,
            vec![("Depth", "1")],
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::MULTI_STATUS);

    // 2. PUT Create Event
    let event_uid = uuid::Uuid::new_v4().to_string();
    let ics_body = format!(
        "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nUID:{}\nSUMMARY:Test Event\nDTSTART:20240101T000000Z\nDTEND:20240101T010000Z\nEND:VEVENT\nEND:VCALENDAR",
        event_uid
    );
    let response = app
        .clone()
        .oneshot(create_request(
            "PUT",
            format!("/caldav/{}/{}.ics", telegram_id, event_uid),
            &auth_header,
            vec![],
            Body::from(ics_body.clone()),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let etag_val = response
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 3. GET Event
    let response = app
        .clone()
        .oneshot(create_request(
            "GET",
            format!("/caldav/{}/{}.ics", telegram_id, event_uid),
            &auth_header,
            vec![],
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 3b. PUT Update Event (with If-Match)
    let updated_ics_body = ics_body.replace("SUMMARY:Test Event", "SUMMARY:Updated Test Event");
    let response = app
        .clone()
        .oneshot(create_request(
            "PUT",
            format!("/caldav/{}/{}.ics", telegram_id, event_uid),
            &auth_header,
            vec![("If-Match", &etag_val)],
            Body::from(updated_ics_body),
        ))
        .await
        .unwrap();
    assert!(response.status().is_success());
    let new_etag_val = response
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_ne!(etag_val, new_etag_val);

    // 4. REPORT Calendar Query
    let report_body = r#"<C:calendar-query xmlns:C="urn:ietf:params:xml:ns:caldav">
<C:filter>
<C:comp-filter name="VCALENDAR">
<C:comp-filter name="VEVENT"/>
</C:comp-filter>
</C:filter>
</C:calendar-query>"#;
    let response = app
        .clone()
        .oneshot(create_request(
            "REPORT",
            format!("/caldav/{}/", telegram_id),
            &auth_header,
            vec![],
            Body::from(report_body),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::MULTI_STATUS);

    // 5. REPORT Sync Collection
    let sync_body = r#"<D:sync-collection xmlns:D="DAV:">
<D:sync-token/>
<D:sync-level>1</D:sync-level>
<D:prop>
    <D:getetag/>
</D:prop>
</D:sync-collection>"#;
    let response = app
        .clone()
        .oneshot(create_request(
            "REPORT",
            format!("/caldav/{}/", telegram_id),
            &auth_header,
            vec![],
            Body::from(sync_body),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::MULTI_STATUS);

    // 5b. REPORT Calendar Multiget
    let multiget_body = format!(
        r#"<C:calendar-multiget xmlns:C="urn:ietf:params:xml:ns:caldav">
<D:prop xmlns:D="DAV:">
<D:getetag/>
<C:calendar-data/>
</D:prop>
<C:href>/caldav/{}/{}.ics</C:href>
</C:calendar-multiget>"#,
        telegram_id, event_uid
    );
    let response = app
        .clone()
        .oneshot(create_request(
            "REPORT",
            format!("/caldav/{}/", telegram_id),
            &auth_header,
            vec![],
            Body::from(multiget_body),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::MULTI_STATUS);

    // 6. DELETE Event
    let response = app
        .clone()
        .oneshot(create_request(
            "DELETE",
            format!("/caldav/{}/{}.ics", telegram_id, event_uid),
            &auth_header,
            vec![("If-Match", &new_etag_val)],
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 7. Verify Deletion
    let response = app
        .clone()
        .oneshot(create_request(
            "GET",
            format!("/caldav/{}/{}.ics", telegram_id, event_uid),
            &auth_header,
            vec![],
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
