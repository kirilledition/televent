use crate::config::API_BURST_SIZE;
use crate::config::API_PERIOD_MS;
use crate::config::CALDAV_BURST_SIZE;
use crate::config::CALDAV_PERIOD_MS;
use crate::docs::ApiDoc;
use crate::middleware::security_headers::security_headers;
use crate::middleware::telegram_auth::telegram_auth;
use crate::middleware::caldav_auth::caldav_basic_auth;
use axum::{
    Router,
    extract::FromRef,
    middleware as axum_middleware,
};
use moka::future::Cache;
use sqlx::PgPool;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder, key_extractor::KeyExtractor, tower_governor::errors::GovernorError};
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod config;
pub mod db;
pub mod docs;
pub mod error;
pub mod middleware;
pub mod models;
pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth_cache: Cache<String, ()>,
    pub telegram_bot_token: String,
}

// Extract IP for rate limiting
#[derive(Clone, Copy, Debug)]
struct UserOrIpKeyExtractor;

impl KeyExtractor for UserOrIpKeyExtractor {
    type Key = String;

    fn extract<B>(&self, req: &axum::http::Request<B>) -> Result<Self::Key, GovernorError> {
        // 1. Try to get User ID from extensions
        if let Some(user_id) = req.extensions().get::<televent_core::models::UserId>() {
            return Ok(format!("user:{}", user_id));
        }

        // 2. Fallback to IP address
        if let Some(ip) = req
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        {
            return Ok(format!("ip:{}", ip.0.ip()));
        }

        // 3. Fallback for unknown
        Ok("unknown".to_string())
    }
}

pub struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            if !components.security_schemes.contains_key("telegram_auth") {
                components.add_security_scheme(
                    "telegram_auth",
                    utoipa::openapi::security::SecurityScheme::ApiKey(
                        utoipa::openapi::security::ApiKey::Header(
                            utoipa::openapi::security::ApiKeyValue::new("x-telegram-init-data"),
                        ),
                    ),
                );
            }
        }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

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
                panic!("Invalid CORS origin configuration: {}", e);
            }
        }
    };

    Router::new()
        .merge(routes::health::routes())
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
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
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    caldav_basic_auth,
                ))
                .layer(GovernorLayer::new(
                    GovernorConfigBuilder::default()
                        .period(std::time::Duration::from_millis(CALDAV_PERIOD_MS))
                        .burst_size(CALDAV_BURST_SIZE)
                        .key_extractor(UserOrIpKeyExtractor)
                        .finish()
                        .expect("Failed to create CalDAV governor config"),
                ))
                .layer(axum_middleware::from_fn(
                    crate::middleware::caldav_logging::caldav_logger,
                )),
        )
        .nest_service(
            "/app",
            ServeDir::new("../frontend/out")
                .not_found_service(ServeFile::new("../frontend/out/index.html")),
        )
        .layer(cors)
        .layer(axum_middleware::from_fn(
            crate::middleware::caldav_headers::add_caldav_headers,
        ))
        .layer(axum_middleware::from_fn(security_headers))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn export_openapi_json() {
        let openapi = ApiDoc::openapi();
        let json = openapi
            .to_pretty_json()
            .expect("Failed to serialize OpenAPI to JSON");

        let path = Path::new("../docs/openapi.json");
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if std::fs::create_dir_all(parent).is_err() {
                    eprintln!("Warning: Could not create docs directory.");
                    return;
                }
            }
        }

        let mut file = File::create(path).expect("Failed to create openapi.json");
        file.write_all(json.as_bytes())
            .expect("Failed to write openapi.json");
    }

    #[tokio::test]
    async fn test_cors_configuration() {
        use axum::{
            body::Body,
            http::{Method, Request, header},
        };
        use tower::ServiceExt;

        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/dummy").unwrap();
        let auth_cache = moka::future::Cache::builder().build();
        let state = AppState {
            pool,
            auth_cache,
            telegram_bot_token: "dummy".to_string(),
        };

        let app = create_router(state.clone(), "*");
    }
}
