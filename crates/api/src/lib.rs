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
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::middleware::caldav_auth::{LoginId, caldav_basic_auth};
use crate::middleware::rate_limit::{
    API_BURST_SIZE, API_PERIOD_MS, CALDAV_BURST_SIZE, CALDAV_PERIOD_MS, UserOrIpKeyExtractor,
};
use crate::middleware::telegram_auth::telegram_auth;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth_cache: Cache<(LoginId, String), Uuid>,
    pub telegram_bot_token: String,
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
        .nest(
            "/api",
            routes::events::routes()
                .merge(routes::calendars::routes())
                .merge(routes::devices::routes())
                .merge(routes::me::routes())
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    telegram_auth,
                ))
                .layer(GovernorLayer::new(
                    GovernorConfigBuilder::default()
                        .period(std::time::Duration::from_millis(API_PERIOD_MS))
                        .burst_size(API_BURST_SIZE)
                        .key_extractor(UserOrIpKeyExtractor)
                        .finish()
                        .expect("Failed to create API governor config"),
                )),
        )
        .nest(
            "/caldav",
            routes::caldav::routes()
                .layer(GovernorLayer::new(
                    GovernorConfigBuilder::default()
                        .period(std::time::Duration::from_millis(CALDAV_PERIOD_MS))
                        .burst_size(CALDAV_BURST_SIZE)
                        .key_extractor(UserOrIpKeyExtractor)
                        .finish()
                        .expect("Failed to create CalDAV governor config"),
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    caldav_basic_auth,
                ))
                .layer(axum_middleware::from_fn(
                    crate::middleware::caldav_logging::caldav_logger,
                )),
        )
        // Serve frontend static files with SPA fallback
        .nest_service(
            "/app",
            ServeDir::new("frontend/out")
                .not_found_service(ServeFile::new("frontend/out/index.html")),
        )
        .layer(cors)
        .layer(axum_middleware::from_fn(
            crate::middleware::caldav_headers::add_caldav_headers,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    let remote_addr = request
                        .extensions()
                        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                        .map(|ci| ci.0.to_string())
                        .unwrap_or_else(|| "unknown".into());

                    let user_agent = request
                        .headers()
                        .get(axum::http::header::USER_AGENT)
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("unknown");

                    let forwarded_for = request
                        .headers()
                        .get("x-forwarded-for")
                        .and_then(|h| h.to_str().ok());

                    tracing::info_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                        version = ?request.version(),
                        remote_addr = %remote_addr,
                        forwarded_for = ?forwarded_for,
                        user_agent = %user_agent,
                    )
                })
                .on_request(|_request: &axum::http::Request<_>, _span: &tracing::Span| {
                    tracing::info!("started processing request");
                })
                .on_response(
                    |response: &axum::http::Response<_>,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::info!(
                            latency_ms = %latency.as_millis(),
                            status = %response.status(),
                            "finished processing request"
                        );
                    },
                ),
        )
        .with_state(state)
}

/// Run the API server
///
/// This function starts the HTTP server and blocks until it exits.
///
/// # Arguments
/// * `state` - Application state containing database pool and caches
/// * `config` - Server configuration
pub async fn run_api(state: AppState, config: &config::Config) -> Result<(), std::io::Error> {
    let app = create_router(state, &config.cors_allowed_origin);
    let addr = format!("{}:{}", config.host, config.port);

    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
}
