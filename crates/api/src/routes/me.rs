use crate::middleware::telegram_auth::AuthenticatedTelegramUser;
use axum::{Extension, Json};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub telegram_id: i64,
    pub username: Option<String>,
    pub authenticated: bool,
    pub timezone: String,
}

/// Get current user profile
///
/// Returns the authenticated user's profile based on Telegram initData.
async fn get_me(Extension(auth_user): Extension<AuthenticatedTelegramUser>) -> Json<MeResponse> {
    Json(MeResponse {
        id: auth_user.id,
        telegram_id: auth_user.telegram_id,
        username: auth_user.username,
        authenticated: true,
        timezone: auth_user.timezone,
    })
}

pub fn routes() -> axum::Router<crate::AppState> {
    axum::Router::new().route("/me", axum::routing::get(get_me))
}
