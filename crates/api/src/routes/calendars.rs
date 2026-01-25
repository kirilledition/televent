//! Calendar management endpoints

use axum::{
    Extension, Json, Router,
    extract::{FromRef, State},
    routing::get,
};
use sqlx::PgPool;
use televent_core::models::Calendar;

use crate::{db, error::ApiError, middleware::telegram_auth::AuthenticatedTelegramUser};

/// List user's calendars
///
/// Currently returns a list containing the single user calendar.
/// Automatically creates the calendar if it doesn't exist.
async fn list_calendars(
    State(pool): State<PgPool>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
) -> Result<Json<Vec<Calendar>>, ApiError> {
    // Current design is 1 user = 1 calendar
    // This function ensures the calendar exists and returns it
    // The implementation of get_or_create_calendar handles the logic
    let calendar = db::calendars::get_or_create_calendar(&pool, auth_user.id).await?;

    Ok(Json(vec![calendar]))
}

/// Calendar routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    PgPool: FromRef<S>,
{
    Router::new().route("/calendars", get(list_calendars))
}
