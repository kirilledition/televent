//! Televent API Server Library

pub mod config;
mod db;
pub mod error;
mod middleware;
mod routes;

use axum::extract::FromRef;
use axum::{Router, middleware as axum_middleware};
use moka::future::Cache;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::middleware::caldav_auth::caldav_basic_auth;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth_cache: Cache<(i64, String), Uuid>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

/// Create the application router
pub fn create_router(state: AppState, cors_origin: &str) -> Router {
    let cors = if cors_origin == "*" {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        match cors_origin.parse::<axum::http::HeaderValue>() {
            Ok(origin) => CorsLayer::new()
                .allow_origin(origin)
                .allow_methods(Any)
                .allow_headers(Any),
            Err(e) => {
                tracing::error!("Invalid CORS origin '{}': {}", cors_origin, e);
                // Fallback to strict (no origin allowed) or Any?
                // Safest is to panic or fail startup, but here we return a Router.
                // Let's fallback to allowing nothing effectively by not adding the layer?
                // Or panic since this is startup config.
                panic!("Invalid CORS origin configuration: {}", e);
            }
        }
    };

    Router::new()
        .merge(routes::health::routes())
        .nest("/api", routes::events::routes())
        .nest("/api", routes::devices::routes())
        .nest(
            "/caldav",
            routes::caldav::routes()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    caldav_basic_auth,
                ))
                .layer(axum_middleware::from_fn(
                    crate::middleware::caldav_logging::caldav_logger,
                )),
        )
        .layer(cors)
        .with_state(state)
}
