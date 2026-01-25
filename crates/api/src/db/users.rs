//! User database operations

use sqlx::PgPool;
use televent_core::models::User;

use crate::error::ApiError;

/// Get or create user by Telegram ID
///
/// If user doesn't exist, creates one.
/// Updates username if changed? Maybe later. For now just get/create.
pub async fn get_or_create_user(
    pool: &PgPool,
    telegram_id: i64,
    username: Option<&str>,
) -> Result<User, ApiError> {
    // Try to find user first
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE telegram_id = $1")
        .bind(telegram_id)
        .fetch_optional(pool)
        .await?;

    if let Some(user) = user {
        // Optional: Update username if different?
        return Ok(user);
    }

    // Create user
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (telegram_id, telegram_username, timezone)
        VALUES ($1, $2, 'UTC')
        RETURNING *
        "#,
    )
    .bind(telegram_id)
    .bind(username)
    .fetch_one(pool)
    .await?;

    Ok(user)
}
