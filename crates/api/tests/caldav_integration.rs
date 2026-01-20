use api::{create_router, AppState};
use moka::future::Cache;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt; // for oneshot
use axum::body::Body;
use axum::http::{Request, StatusCode};

#[sqlx::test]
async fn test_caldav_root_propfind(pool: PgPool) {
    // Setup state
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();
    
    let state = AppState { pool, auth_cache };
    let app = create_router(state, "*");

    // Create a request
    // CalDAV PROPFIND on root requires auth, but let's check basic response behavior
    // If we don't provide auth, it should return 401
    
    let response = app
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/caldav/")
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
