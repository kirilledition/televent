use crate::middleware::telegram_auth::AuthenticatedTelegramUser;
use axum::{Extension, Json};
use serde::Serialize;
use televent_core::models::{Timezone, UserId};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    #[schema(value_type = String)]
    pub id: UserId,
    pub username: Option<String>,
    pub authenticated: bool,
    pub timezone: Timezone,
}

/// Get current user profile
///
/// Returns the authenticated user's profile based on Telegram initData.
#[utoipa::path(
    get,
    path = "/me",
    responses(
        (status = 200, description = "Current user profile", body = MeResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "user",
    security(
        ("telegram_auth" = [])
    )
)]
async fn get_me(Extension(auth_user): Extension<AuthenticatedTelegramUser>) -> Json<MeResponse> {
    Json(MeResponse {
        id: auth_user.id,
        username: auth_user.username,
        authenticated: true,
        timezone: auth_user.timezone,
    })
}

pub fn routes() -> axum::Router<crate::AppState> {
    axum::Router::new().route("/me", axum::routing::get(get_me))
}
