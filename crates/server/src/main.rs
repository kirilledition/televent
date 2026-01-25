use anyhow::Result;
use sqlx::PgPool;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Initialize tracing once for entire process
    // The guard must be kept alive for the duration of the program to ensure logs are flushed
    let _guard = init_tracing()?;

    tracing::info!("🚀 Starting Televent unified server");

    // Load unified configuration
    let config = config::UnifiedConfig::from_env()?;
    tracing::info!("✓ Configuration loaded");

    // Create shared database pool with explicit configuration
    let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "50".to_string())
        .parse()
        .unwrap_or(50);

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800)) // 30 minutes
        .connect(&config.core.database_url)
        .await?;
    tracing::info!(
        "✓ Database pool established (max_connections: {})",
        max_connections
    );

    // Run migrations ONCE
    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("✓ Migrations completed");

    // Create shutdown coordination
    let shutdown = CancellationToken::new();

    // Spawn all services
    let api_handle = spawn_api(pool.clone(), config.clone(), shutdown.clone());
    let bot_handle = spawn_bot(pool.clone(), config.clone(), shutdown.clone());
    let worker_handle = spawn_worker(pool.clone(), config.clone(), shutdown.clone());

    tracing::info!("✓ All services started");

    // Wait for shutdown signal
    wait_for_shutdown().await;
    tracing::info!("📡 Shutdown signal received");

    // Cancel all services
    shutdown.cancel();

    // Wait for graceful shutdown
    let _ = tokio::join!(api_handle, bot_handle, worker_handle);

    tracing::info!("✓ All services stopped gracefully");
    Ok(())
}

fn spawn_api(
    pool: PgPool,
    config: config::UnifiedConfig,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let auth_cache = moka::future::Cache::builder()
            .time_to_live(std::time::Duration::from_secs(300))
            .build();

        let state = api::AppState { pool, auth_cache };
        let api_config = config.to_api_config();

        tokio::select! {
            result = api::run_api(state, &api_config) => {
                tracing::error!("API service exited: {:?}", result);
                result.map_err(|e| anyhow::anyhow!(e))
            }
            _ = shutdown.cancelled() => {
                tracing::info!("API service shutting down");
                Ok(())
            }
        }
    })
}

fn spawn_bot(
    pool: PgPool,
    config: config::UnifiedConfig,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let bot_token = config.core.telegram_bot_token.clone();

        tokio::select! {
            result = bot::run_bot(pool, bot_token) => {
                tracing::error!("Bot service exited: {:?}", result);
                result
            }
            _ = shutdown.cancelled() => {
                tracing::info!("Bot service shutting down");
                Ok(())
            }
        }
    })
}

fn spawn_worker(
    pool: PgPool,
    config: config::UnifiedConfig,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let bot = teloxide::Bot::new(&config.core.telegram_bot_token);
        let worker_config = config.to_worker_config();

        worker::run_worker(pool, bot, worker_config, Some(shutdown)).await
    })
}

async fn wait_for_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn init_tracing() -> Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,api=debug,bot=debug,worker=debug,sqlx=warn".into());

    let stdout_layer = tracing_subscriber::fmt::layer().with_target(true);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer);

    let enable_file_logging = std::env::var("ENABLE_FILE_LOGGING")
        .map(|v| v.to_lowercase() != "false" && v != "0")
        .unwrap_or(true);

    if enable_file_logging {
        let file_appender = tracing_appender::rolling::daily("logs", "televent.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(non_blocking)
            .json();

        registry.with(file_layer).init();

        Ok(Some(guard))
    } else {
        registry.init();
        Ok(None)
    }
}
