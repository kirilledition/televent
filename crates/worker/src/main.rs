use anyhow::Result;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "worker=debug".into()),
        )
        .init();

    info!("Starting Televent worker");

    // TODO: Implement outbox consumer with:
    // - Database connection pool
    // - Polling loop with FOR UPDATE SKIP LOCKED
    // - Email sender integration
    // - Telegram notification sender
    // - Retry logic with exponential backoff
    // - Dead-letter handling

    info!("Worker ready (stub implementation)");

    loop {
        // TODO: Poll outbox_messages table
        // Process pending jobs
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
