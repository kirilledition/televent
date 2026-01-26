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
use televent_core::models::UserId;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::middleware::caldav_auth::{LoginId, caldav_basic_auth};
use crate::middleware::rate_limit::{
    API_BURST_SIZE, API_PERIOD_MS, CALDAV_BURST_SIZE, CALDAV_PERIOD_MS, UserOrIpKeyExtractor,
};
use crate::middleware::telegram_auth::telegram_auth;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth_cache: Cache<(LoginId, String), UserId>,
    pub telegram_bot_token: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health::health_check,
        routes::me::get_me,
        routes::events::create_event,
        routes::events::list_events,
        routes::events::get_event,
        routes::events::update_event,
        routes::events::delete_event_handler,
        routes::calendars::list_calendars,
        routes::devices::create_device_password,
        routes::devices::list_device_passwords,
        routes::devices::delete_device_password,
    ),
    components(
        schemas(
            televent_core::models::UserId,
            televent_core::models::Timezone,
            televent_core::models::User,
            televent_core::models::Event,
            televent_core::models::EventStatus,
            televent_core::models::EventAttendee,
            televent_core::models::AttendeeRole,
            televent_core::models::ParticipationStatus,
            routes::health::HealthResponse,
            routes::me::MeResponse,
            routes::events::CreateEventRequest,
            routes::events::UpdateEventRequest,
            routes::events::ListEventsQuery,
            routes::calendars::CalendarInfo,
            routes::devices::CreateDeviceRequest,
            routes::devices::DevicePasswordResponse,
            routes::devices::DeviceListItem,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "user", description = "User profile endpoints"),
        (name = "events", description = "Event management endpoints"),
        (name = "calendars", description = "Calendar management endpoints"),
        (name = "devices", description = "Device management endpoints"),
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
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
    } else if cors_origin == "mirror" {
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(|_: &_, _: &_| true))
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_credentials(true)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn export_openapi_json() {
        let openapi = ApiDoc::openapi();
        let json = openapi
            .to_pretty_json()
            .expect("Failed to serialize OpenAPI to JSON");

        let path = "../../openapi.json";
        let mut file = File::create(path).expect("Failed to create openapi.json");
        file.write_all(json.as_bytes())
            .expect("Failed to write openapi.json");

        println!("OpenAPI JSON exported to {}", path);
    }
}
