use crate::config::API_BURST_SIZE;
use crate::config::API_PERIOD_MS;
use crate::config::CALDAV_BURST_SIZE;
use crate::config::CALDAV_PERIOD_MS;
use crate::docs::ApiDoc;
use crate::middleware::security_headers;
use crate::middleware::telegram_auth;
use axum::{
    Router,
    extract::FromRef,
    middleware as axum_middleware,
};
use axum_client_ip::SecureClientIpSource;
use moka::future::Cache;
use sqlx::PgPool;
use std::sync::Arc;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder, key_extractor::KeyExtractor};
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
    pub auth_cache: Cache<String, ()>, // Cache for device password hashes (key: "<user_id>:<password>")
    pub telegram_bot_token: String,
}

// Extract IP for rate limiting
#[derive(Clone, Copy, Debug)]
struct UserOrIpKeyExtractor;

impl KeyExtractor for UserOrIpKeyExtractor {
    type Key = String;

    fn extract<B>(&self, req: &axum::http::Request<B>) -> Result<Self::Key, tower_governor::governor::Quota> {
        // 1. Try to get User ID from extensions (set by auth middleware)
        if let Some(user_id) = req.extensions().get::<televent_core::models::UserId>() {
            return Ok(format!("user:{}", user_id));
        }

        // 2. Fallback to IP address
        // Using SecureClientIpSource from axum-client-ip would be better but requires more setup
        // For now, we trust the ConnectInfo or X-Forwarded-For if properly configured
        if let Some(ip) = req
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        {
            return Ok(format!("ip:{}", ip.0.ip()));
        }

        // 3. Fallback for unknown (should be rare with correct setup)
        Ok("unknown".to_string())
    }
}

pub fn add_security_scheme(openapi: &mut utoipa::openapi::OpenApi) {
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
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest(
            "/api",
            routes::events::routes()
                .merge(routes::calendars::routes())
                .merge(routes::devices::routes())
                .merge(routes::me::routes())
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::telegram_auth::telegram_auth,
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
                    middleware::caldav_auth::caldav_basic_auth,
                ))
                // Rate limit BEFORE auth to prevent Argon2 CPU exhaustion attacks (DoS)
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
        // Serve frontend static files with SPA fallback
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

/// Run the API server
///
/// This function starts the HTTP server and blocks until it exits.
///
/// # Arguments
/// *  - Application state containing database pool and caches
/// *  - Server configuration
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

        // Use a relative path from CARGO_MANIFEST_DIR to locate the docs folder
        // backend/api/ -> backend/docs/
        // Actually, the structure is backend/api and backend/docs is probably at backend/../docs?
        // Wait, 'docs' folder is not in 'backend'.
        // Repository structure:
        // backend/
        //   api/
        // docs/ (maybe?)
        // Let's assume we just want to output it if the directory exists, or skip it.
        // Or write to OUT_DIR if it's for build.
        // But this is a test.
        // Let's try to find the project root.

        let path = Path::new("../docs/openapi.json");
        if let Some(parent) = path.parent() {
            if !parent.exists() {
               // In CI or some envs, docs might not exist. Create it or skip.
               // For CI, let's try to create it if it doesn't exist, to prevent failure.
               // But usually this test is meant to update the repo file.
               // If we are in CI, maybe we don't need to write it?
               // But let's make it robust by checking directory existence.
               if std::fs::create_dir_all(parent).is_err() {
                   eprintln!("Warning: Could not create docs directory. Skipping openapi.json export.");
                   return;
               }
            }
        }

        let mut file = File::create(path).expect("Failed to create openapi.json");
        file.write_all(json.as_bytes())
            .expect("Failed to write openapi.json");

        println!("OpenAPI JSON exported to {:?}", path);
    }

    #[tokio::test]
    async fn test_cors_configuration() {
        use axum::{
            body::Body,
            http::{Method, Request, header},
        };
        use tower::ServiceExt;

        // Create dummy state
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/dummy").unwrap();
        let auth_cache = moka::future::Cache::builder().build();
        let state = AppState {
            pool,
            auth_cache,
            telegram_bot_token: "dummy".to_string(),
        };

        // Test 1: Wildcard "*"
        let app = create_router(state.clone(), "*");

        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/events")
            .header(header::ORIGIN, "http://evil.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();

        // With "*", Allow-Origin should be "*"
        let allow_origin = response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
        assert_eq!(allow_origin.map(|h| h.to_str().unwrap()), Some("*"));

        // Crucially, Allow-Credentials must NOT be true if Allow-Origin is *
        let allow_creds = response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS);
        assert!(allow_creds.is_none());

        // Test 2: Specific Origin
        let app = create_router(state.clone(), "http://example.com");

        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/events")
            .header(header::ORIGIN, "http://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();

        let allow_origin = response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
        assert_eq!(
            allow_origin.map(|h| h.to_str().unwrap()),
            Some("http://example.com")
        );

        // Test 3: "mirror" should NO LONGER work as a magic value
        // It will be treated as a literal origin "mirror", which won't match "http://evil.com"
        let app = create_router(state.clone(), "mirror");

        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/events")
            .header(header::ORIGIN, "http://evil.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();

        // Should NOT allow evil.com
        let allow_origin = response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
        assert_ne!(
            allow_origin.map(|h| h.to_str().unwrap()),
            Some("http://evil.com")
        );
        // And definitely not *
        assert_ne!(allow_origin.map(|h| h.to_str().unwrap()), Some("*"));
    }
}
