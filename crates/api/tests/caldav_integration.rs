use api::{AppState, create_router};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use moka::future::Cache;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt; // for oneshot

#[sqlx::test]
async fn test_caldav_root_propfind(pool: PgPool) {
    // Setup state
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState { pool, auth_cache };
    let app = create_router(state, "*");

    // Create a request
    // CalDAV PROPFIND on user calendar requires auth
    // If we don't provide auth, it should return 401
    // Use a random UUID since auth will fail anyway

    let test_user_id = uuid::Uuid::new_v4();
    let response = app
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri(format!("/caldav/{}/", test_user_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_health_check(pool: PgPool) {
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState { pool, auth_cache };
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
