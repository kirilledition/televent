//! User database operations

use sqlx::PgPool;
use televent_core::models::{User, UserId};

use crate::error::ApiError;

/// Get or create user by Telegram ID
///
/// If user doesn't exist, creates one with default calendar settings.
/// Returns the user with merged calendar fields.
pub async fn get_or_create_user(
    pool: &PgPool,
    telegram_id: i64,
    username: Option<&str>,
) -> Result<User, ApiError> {
    // Use upsert to atomically get or create
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (telegram_id, telegram_username)
        VALUES ($1, $2)
        ON CONFLICT (telegram_id) DO UPDATE
        SET telegram_username = COALESCE(EXCLUDED.telegram_username, users.telegram_username)
        RETURNING *
        "#,
    )
    .bind(telegram_id)
    .bind(username)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Get user by Telegram ID
pub async fn get_user_by_id(pool: &PgPool, user_id: UserId) -> Result<Option<User>, ApiError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE telegram_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

/// Get user by Telegram username (case-insensitive)
pub async fn get_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>, ApiError> {
    let user =
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE lower(telegram_username) = lower($1)")
            .bind(username)
            .fetch_optional(pool)
            .await?;

    Ok(user)
}

/// Increment sync token for a user's calendar
pub async fn increment_sync_token(pool: &PgPool, user_id: UserId) -> Result<String, ApiError> {
    let result = sqlx::query_scalar::<_, String>(
        "UPDATE users
         SET sync_token = (sync_token::bigint + 1)::text,
             ctag = EXTRACT(EPOCH FROM NOW())::text,
             updated_at = NOW()
         WHERE telegram_id = $1
         RETURNING sync_token",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(result)
}

/// Increment sync token for a user's calendar (within transaction)
pub async fn increment_sync_token_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: UserId,
) -> Result<String, ApiError> {
    let result = sqlx::query_scalar::<_, String>(
        "UPDATE users
         SET sync_token = (sync_token::bigint + 1)::text,
             ctag = EXTRACT(EPOCH FROM NOW())::text,
             updated_at = NOW()
         WHERE telegram_id = $1
         RETURNING sync_token",
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(result)
}

/// Update user's timezone
#[allow(dead_code)]
pub async fn update_timezone(
    pool: &PgPool,
    user_id: UserId,
    timezone: &str,
) -> Result<User, ApiError> {
    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET timezone = $2, updated_at = NOW() WHERE telegram_id = $1 RETURNING *",
    )
    .bind(user_id)
    .bind(timezone)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Update user's calendar name
#[allow(dead_code)]
pub async fn update_calendar_name(
    pool: &PgPool,
    user_id: UserId,
    name: &str,
) -> Result<User, ApiError> {
    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET calendar_name = $2, updated_at = NOW() WHERE telegram_id = $1 RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .fetch_one(pool)
    .await?;

    Ok(user)
}
