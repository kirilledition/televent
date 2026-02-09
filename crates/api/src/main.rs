use anyhow::Result;
use api::AppState;
use moka::future::Cache;
use std::time::Duration;
use televent_shared::bootstrap;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Init env
    bootstrap::init_env();

    // 2. Init tracing
    let _guard = bootstrap::init_tracing("api");

    tracing::info!("Starting Televent API server");

    // 3. Load config
    let config = api::config::Config::from_env()?;
    tracing::info!(
        "Server configuration loaded: {}:{}",
        config.host,
        config.port
    );

    // 4. Init DB
    let pool = bootstrap::init_db(&config.core).await?;

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Initialize Auth Cache (TTL: 5 minutes)
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();

    let state = AppState {
        pool,
        auth_cache,
        telegram_bot_token: config.telegram_bot_token.clone(),
    };

    // Start server using library function
    api::run_api(state, &config).await?;

    Ok(())
}
