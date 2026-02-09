//! Calendar management endpoints
//!
//! Since each user has exactly one calendar (user = calendar), these endpoints
//! return user calendar properties.

use axum::{
    Extension, Json, Router,
    extract::{FromRef, State},
    routing::get,
};
use serde::Serialize;
use sqlx::PgPool;
use televent_core::models::{CALENDAR_COLOR, CALENDAR_NAME, UserId};
use utoipa::ToSchema;

use crate::{db, error::ApiError, middleware::telegram_auth::AuthenticatedTelegramUser};

/// Calendar response (subset of User relevant to calendar functionality)
#[derive(Debug, Serialize, ToSchema)]
pub struct CalendarInfo {
    #[schema(value_type = String)]
    pub id: UserId,
    pub name: String,
    pub color: String,
    pub sync_token: String,
}

/// List user's calendars
///
/// Returns a list containing the single user calendar.
/// Since user = calendar, this returns calendar properties from the user record.
#[utoipa::path(
    get,
    path = "/calendars",
    responses(
        (status = 200, description = "List of calendars", body = Vec<CalendarInfo>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "calendars",
    security(
        ("telegram_auth" = [])
    )
)]
async fn list_calendars(
    State(pool): State<PgPool>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
) -> Result<Json<Vec<CalendarInfo>>, ApiError> {
    // Get or create user (user = calendar)
    let user = db::users::get_or_create_user(&pool, auth_user.id.inner(), None).await?;

    let calendar_info = CalendarInfo {
        id: user.id,
        name: CALENDAR_NAME.to_string(),
        color: CALENDAR_COLOR.to_string(),
        sync_token: user.sync_token,
    };

    Ok(Json(vec![calendar_info]))
}

/// Calendar routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    PgPool: FromRef<S>,
{
    Router::new().route("/calendars", get(list_calendars))
}
