use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bot=debug".into()),
        )
        .init();

    info!("Starting Televent Telegram bot");

    // TODO: Implement Teloxide bot with:
    // - Bot initialization from env
    // - Command handlers (/start, /today, /create, etc.)
    // - Database integration
    // - FSM for multi-step flows
    // - Telegram OAuth validation

    info!("Bot ready (stub implementation)");

    // Keep the process alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down bot");

    Ok(())
}
