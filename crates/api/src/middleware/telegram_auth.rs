use crate::AppState;
use crate::error::ApiError;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use televent_core::models::{Timezone, UserId};

// Constants
const AUTH_HEADER_PREFIX: &str = "tma ";

/// User information extracted from Telegram initData
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TelegramUser {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: Option<String>,
    pub is_premium: Option<bool>,
    pub allows_write_to_pm: Option<bool>,
}
// Extension type to hold authenticated user
#[derive(Debug, Clone)]
pub struct AuthenticatedTelegramUser {
    pub id: UserId,
    pub username: Option<String>,
    pub timezone: Timezone,
}

/// Middleware to validate Telegram initData and inject TelegramUser into extensions
pub async fn telegram_auth(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, ApiError> {
    // Check Authorization header
    let auth_header = match request.headers().get(axum::http::header::AUTHORIZATION) {
        Some(h) => h.to_str().unwrap_or(""),
        None => {
            return Err(ApiError::Unauthorized(
                "Missing Authorization header".into(),
            ));
        }
    };

    if !auth_header.starts_with(AUTH_HEADER_PREFIX) {
        return Err(ApiError::Unauthorized(
            "Invalid Authorization scheme".into(),
        ));
    }

    let init_data = &auth_header[AUTH_HEADER_PREFIX.len()..];

    // Parse query string
    let parsed: HashMap<String, String> = url::form_urlencoded::parse(init_data.as_bytes())
        .into_owned()
        .collect();

    let hash = parsed
        .get("hash")
        .ok_or(ApiError::Unauthorized("Missing hash".into()))?;

    // Validate signature
    // Implementation of data-check-string construction
    // Specify type explicitly to avoid inference error
    let mut keys: Vec<&String> = parsed.keys().filter(|k| k.as_str() != "hash").collect();
    keys.sort();

    let mut data_check_string = String::new();
    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            data_check_string.push('\n');
        }
        data_check_string.push_str(key);
        data_check_string.push('=');
        data_check_string.push_str(&parsed[*key]);
    }

    let bot_token = &state.telegram_bot_token;

    // HMAC-SHA256 signature
    type HmacSha256 = Hmac<Sha256>;

    let secret_key = HmacSha256::new_from_slice(b"WebAppData")
        .expect("HMAC can take any key length")
        .chain_update(bot_token.as_bytes())
        .finalize()
        .into_bytes();

    let mut mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC can take any key length");
    mac.update(data_check_string.as_bytes());

    let result = mac.finalize();
    let calculated_hash = hex::encode(result.into_bytes());

    // Skip validation for now if needed, OR enforce it.
    // Since we have the token, let's enforce it to be safe, or log warning if it fails but proceed?
    // The previous code ENFORCED it. I will keep enforcing it.
    if calculated_hash != *hash {
        return Err(ApiError::Unauthorized("Invalid signature".into()));
    }

    // Parse User
    let user_json = parsed
        .get("user")
        .ok_or(ApiError::Unauthorized("Missing user data".into()))?;
    let user: TelegramUser = serde_json::from_str(user_json)
        .map_err(|_| ApiError::BadRequest("Invalid user JSON".into()))?;

    // Get/Create User in DB
    let username = user.username.as_deref();
    let db_user = crate::db::users::get_or_create_user(&state.pool, user.id, username)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get/create user: {:?}", e);
            ApiError::Internal("Database error".into())
        })?;

    tracing::info!(
        telegram_id = user.id,
        user_id = %db_user.id,
        "Telegram user authenticated"
    );

    // Success - insert into extensions
    // We insert 2 things:
    // 1. TelegramUser (for AuthUser extractor if needed)
    // 2. AuthenticatedTelegramUser (which contains DB ID, used by endpoints)

    request.extensions_mut().insert(user);
    request.extensions_mut().insert(AuthenticatedTelegramUser {
        id: db_user.id,
        username: db_user.telegram_username,
        timezone: db_user.timezone,
    });

    Ok(next.run(request).await)
}
