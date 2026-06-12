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
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.runtime.db_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800)) // 30 minutes
        .connect(&config.runtime.database_url)
        .await?;
    tracing::info!(
        "✓ Database pool established (max_connections: {})",
        config.runtime.db_max_connections
    );

    // Run migrations ONCE
    sqlx::migrate!("../migrations").run(&pool).await?;
    tracing::info!("✓ Migrations completed");

    // Create shutdown coordination
    let shutdown = CancellationToken::new();

    // Spawn all services
    let mut api_handle = spawn_api(pool.clone(), config.clone(), shutdown.clone());
    let mut bot_handle = spawn_bot(pool.clone(), config.clone(), shutdown.clone());
    let mut worker_handle = spawn_worker(pool.clone(), config.clone(), shutdown.clone());

    tracing::info!("✓ All services started");

    tokio::select! {
        _ = wait_for_shutdown() => {
            tracing::info!("📡 Shutdown signal received");
            shutdown.cancel();
            let _ = tokio::join!(api_handle, bot_handle, worker_handle);
            tracing::info!("✓ All services stopped gracefully");
            Ok(())
        }
        result = &mut api_handle => {
            shutdown.cancel();
            let _ = tokio::join!(bot_handle, worker_handle);
            service_exit_error("API", result)
        }
        result = &mut bot_handle => {
            shutdown.cancel();
            let _ = tokio::join!(api_handle, worker_handle);
            service_exit_error("bot", result)
        }
        result = &mut worker_handle => {
            shutdown.cancel();
            let _ = tokio::join!(api_handle, bot_handle);
            service_exit_error("worker", result)
        }
    }
}

fn service_exit_error(
    service_name: &str,
    result: Result<Result<()>, tokio::task::JoinError>,
) -> Result<()> {
    match result {
        Ok(Ok(())) => Err(anyhow::anyhow!(
            "{} service exited unexpectedly",
            service_name
        )),
        Ok(Err(e)) => Err(anyhow::anyhow!("{} service failed: {}", service_name, e)),
        Err(e) => Err(anyhow::anyhow!("{} task join failed: {}", service_name, e)),
    }
}

fn spawn_api(
    pool: PgPool,
    config: config::UnifiedConfig,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let auth_cache = moka::future::Cache::builder()
            .time_to_live(std::time::Duration::from_secs(300))
            .max_capacity(10000)
            .build();

        let state = api::AppState {
            calendar_service: televent_application::CalendarService::new(
                televent_storage::calendar::CalendarRepository::new(pool.clone()),
            ),
            device_service: televent_application::DeviceService::new(
                televent_storage::device::DeviceRepository::new(pool.clone()),
            ),
            health_service: televent_application::HealthService::new(
                televent_storage::health::HealthRepository::new(pool.clone()),
            ),
            auth_cache,
            telegram_bot_token: config.runtime.telegram_bot_token.clone(),
        };
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
        let bot_token = config.runtime.telegram_bot_token.clone();
        let bot_db = bot::db::BotDb::new(
            televent_application::CalendarService::new(
                televent_storage::calendar::CalendarRepository::new(pool.clone()),
            ),
            televent_application::DeviceService::new(
                televent_storage::device::DeviceRepository::new(pool.clone()),
            ),
        );

        tokio::select! {
            result = bot::run_bot(bot_db, bot_token) => {
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
        let bot = teloxide::Bot::new(&config.runtime.telegram_bot_token);
        let worker_config = config.to_worker_config();
        let db = worker::WorkerDb::new(pool.clone());
        let calendar = televent_application::CalendarService::new(
            televent_storage::calendar::CalendarRepository::new(pool.clone()),
        );

        worker::run_worker(db, calendar, bot, worker_config, Some(shutdown)).await
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

    let running_on_railway = std::env::var("RAILWAY_ENVIRONMENT").is_ok();
    let production = std::env::var("APP_ENV")
        .or_else(|_| std::env::var("RUST_ENV"))
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false);
    let enable_file_logging = std::env::var("ENABLE_FILE_LOGGING")
        .map(|v| v.to_lowercase() != "false" && v != "0")
        .unwrap_or(!running_on_railway && !production);

    if enable_file_logging {
        let now = chrono::Local::now().format("%y-%m-%d-%H-%M-%S").to_string();
        let filename = format!("televent.log.{}.jsonl", now);
        let file_appender = tracing_appender::rolling::never("logs/app", filename);
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
