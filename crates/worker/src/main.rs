//! Televent Worker - Background job consumer
//!
//! This crate processes outbox messages for notifications and async tasks.

use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Televent worker starting...");

    // TODO: Connect to database
    // TODO: Poll outbox_messages table
    // TODO: Process messages with FOR UPDATE SKIP LOCKED
    // TODO: Send notifications via mailer/telegram

    info!("Televent worker initialized (placeholder implementation)");
}
