//! Televent API Server
//!
//! Axum-based web server providing:
//! - CalDAV endpoints for calendar sync
//! - REST API for event management
//! - Authentication via Telegram OAuth and device passwords

mod config;
mod db;
mod error;
mod middleware;
mod routes;

use anyhow::Result;
use axum::Router;
use sqlx::PgPool;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,api=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Televent API server");

    // Load configuration
    let config = config::Config::from_env()?;
    tracing::info!(
        "Server configuration loaded: {}:{}",
        config.host,
        config.port
    );

    // Create database connection pool
    let pool = PgPool::connect(&config.database_url).await?;
    tracing::info!("Database connection pool established");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Build application router
    let app = Router::new()
        .nest("/", routes::health::routes())
        .nest("/api", routes::events::routes())
        .nest("/caldav", routes::caldav::routes())
        .with_state(pool);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
