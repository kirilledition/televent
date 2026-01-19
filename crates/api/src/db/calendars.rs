//! Calendar database operations

use sqlx::PgPool;
use televent_core::models::Calendar;
use uuid::Uuid;

use crate::error::ApiError;

/// Get or create calendar for a user
///
/// Each user has exactly one calendar (enforced by unique index).
/// Uses INSERT ... ON CONFLICT to prevent race conditions.
pub async fn get_or_create_calendar(pool: &PgPool, user_id: Uuid) -> Result<Calendar, ApiError> {
    // Use upsert to atomically get or create - prevents race conditions
    let calendar = sqlx::query_as::<_, Calendar>(
        r#"
        INSERT INTO calendars (user_id, name, color, sync_token, ctag)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_id) DO UPDATE SET user_id = calendars.user_id
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind("My Calendar")
    .bind("#3b82f6")
    .bind("0")
    .bind("0")
    .fetch_one(pool)
    .await?;

    Ok(calendar)
}

/// Get calendar by user_id
pub async fn get_calendar_by_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<Calendar>, ApiError> {
    let calendar = sqlx::query_as::<_, Calendar>("SELECT * FROM calendars WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    Ok(calendar)
}

/// Increment sync token for a calendar
pub async fn increment_sync_token(pool: &PgPool, calendar_id: Uuid) -> Result<String, ApiError> {
    let result = sqlx::query_scalar::<_, String>(
        "UPDATE calendars
         SET sync_token = (sync_token::bigint + 1)::text,
             ctag = EXTRACT(EPOCH FROM NOW())::text
         WHERE id = $1
         RETURNING sync_token",
    )
    .bind(calendar_id)
    .fetch_one(pool)
    .await?;

    Ok(result)
}
