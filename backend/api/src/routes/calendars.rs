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
use televent_application::CalendarService;
use televent_domain::{CALENDAR_COLOR, CALENDAR_NAME};
use utoipa::ToSchema;

use crate::{error::ApiError, middleware::telegram_auth::AuthenticatedTelegramUser};

/// Calendar response (subset of User relevant to calendar functionality)
#[derive(Debug, Serialize, ToSchema)]
pub struct CalendarInfo {
    pub id: String,
    pub name: String,
    pub color: String,
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
    State(calendar): State<CalendarService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
) -> Result<Json<Vec<CalendarInfo>>, ApiError> {
    calendar
        .ensure_user_setup(auth_user.id.inner(), auth_user.username.as_deref())
        .await?;

    let calendar_info = CalendarInfo {
        id: auth_user.id.to_string(),
        name: CALENDAR_NAME.to_string(),
        color: CALENDAR_COLOR.to_string(),
    };

    Ok(Json(vec![calendar_info]))
}

/// Calendar routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    CalendarService: FromRef<S>,
{
    Router::new().route("/calendars", get(list_calendars))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_info_hides_caldav_sync_token() {
        let value = serde_json::to_value(CalendarInfo {
            id: "123".to_string(),
            name: CALENDAR_NAME.to_string(),
            color: CALENDAR_COLOR.to_string(),
        })
        .unwrap();
        let object = value.as_object().unwrap();

        assert!(!object.contains_key("sync_token"));
    }
}
