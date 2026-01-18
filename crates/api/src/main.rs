use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug,tower_http=debug".into()),
        )
        .init();

    info!("Starting Televent API server");

    // TODO: Implement Axum server with:
    // - Database connection pool
    // - Authentication middleware
    // - CalDAV endpoints
    // - REST API endpoints
    // - Health check endpoint

    info!("API server ready (stub implementation)");

    // Keep the process alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down API server");

    Ok(())
}
