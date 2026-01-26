use crate::AppState;
use crate::error::ApiError;
use chrono::Utc;
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

/// Helper function to validate Telegram init data and return the user.
///
/// This encapsulates the logic for:
/// 1. Parsing the init_data string
/// 2. Verifying the HMAC-SHA256 signature
/// 3. Verifying the freshness of the auth_date (within 24 hours)
/// 4. Parsing the user JSON
pub fn validate_init_data(init_data: &str, bot_token: &str) -> Result<TelegramUser, ApiError> {
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

    if calculated_hash != *hash {
        return Err(ApiError::Unauthorized("Invalid signature".into()));
    }

    // Validate auth_date freshness
    if let Some(auth_date_str) = parsed.get("auth_date") {
        let auth_date = auth_date_str
            .parse::<i64>()
            .map_err(|_| ApiError::BadRequest("Invalid auth_date format".into()))?;
        let now = Utc::now().timestamp();
        // Check if auth_date is older than 24 hours (86400 seconds)
        if now - auth_date > 86400 {
            return Err(ApiError::Unauthorized("Auth date expired".into()));
        }
        // Check if auth_date is too far in the future (allow 5 minutes clock skew)
        if auth_date - now > 300 {
            return Err(ApiError::Unauthorized("Auth date in the future".into()));
        }
    } else {
        return Err(ApiError::Unauthorized("Missing auth_date".into()));
    }

    // Parse User
    let user_json = parsed
        .get("user")
        .ok_or(ApiError::Unauthorized("Missing user data".into()))?;
    let user: TelegramUser = serde_json::from_str(user_json)
        .map_err(|_| ApiError::BadRequest("Invalid user JSON".into()))?;

    Ok(user)
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
    let user = validate_init_data(init_data, &state.telegram_bot_token)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use urlencoding::encode;

    // Helper to generate a valid hash for testing
    fn generate_init_data(params: &[(&str, &str)], bot_token: &str) -> String {
        let mut keys: Vec<String> = params.iter().map(|(k, _)| k.to_string()).collect();
        keys.sort();

        let mut data_check_string = String::new();
        for (i, key) in keys.iter().enumerate() {
            if i > 0 {
                data_check_string.push('\n');
            }
            let val = params.iter().find(|(k, _)| *k == key).unwrap().1;
            data_check_string.push_str(key);
            data_check_string.push('=');
            data_check_string.push_str(val);
        }

        type HmacSha256 = Hmac<Sha256>;
        let secret_key = HmacSha256::new_from_slice(b"WebAppData")
            .unwrap()
            .chain_update(bot_token.as_bytes())
            .finalize()
            .into_bytes();

        let mut mac = HmacSha256::new_from_slice(&secret_key).unwrap();
        mac.update(data_check_string.as_bytes());
        let hash = hex::encode(mac.finalize().into_bytes());

        // Construct query string
        let mut query = String::new();
        for (k, v) in params {
            if !query.is_empty() {
                query.push('&');
            }
            query.push_str(k);
            query.push('=');
            query.push_str(&encode(v));
        }
        query.push_str("&hash=");
        query.push_str(&hash);
        query
    }

    #[test]
    fn test_validate_init_data_success() {
        let bot_token = "test_token";
        let user_json = r#"{"id":123,"first_name":"Test","last_name":"User"}"#;
        let auth_date = Utc::now().timestamp().to_string();

        let params = vec![
            ("auth_date", auth_date.as_str()),
            ("query_id", "AAGPK..."),
            ("user", user_json),
        ];

        let init_data = generate_init_data(&params, bot_token);
        let result = validate_init_data(&init_data, bot_token);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.id, 123);
    }

    #[test]
    fn test_validate_init_data_expired() {
        let bot_token = "test_token";
        let user_json = r#"{"id":123,"first_name":"Test","last_name":"User"}"#;
        // 25 hours ago
        let auth_date = (Utc::now().timestamp() - 90000).to_string();

        let params = vec![
            ("auth_date", auth_date.as_str()),
            ("query_id", "AAGPK..."),
            ("user", user_json),
        ];

        let init_data = generate_init_data(&params, bot_token);
        let result = validate_init_data(&init_data, bot_token);

        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::Unauthorized(msg) => assert_eq!(msg, "Auth date expired"),
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_validate_init_data_future() {
        let bot_token = "test_token";
        let user_json = r#"{"id":123,"first_name":"Test","last_name":"User"}"#;
        // 10 minutes in future
        let auth_date = (Utc::now().timestamp() + 600).to_string();

        let params = vec![
            ("auth_date", auth_date.as_str()),
            ("query_id", "AAGPK..."),
            ("user", user_json),
        ];

        let init_data = generate_init_data(&params, bot_token);
        let result = validate_init_data(&init_data, bot_token);

        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::Unauthorized(msg) => assert_eq!(msg, "Auth date in the future"),
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_validate_init_data_missing_auth_date() {
        let bot_token = "test_token";
        let user_json = r#"{"id":123,"first_name":"Test","last_name":"User"}"#;

        let params = vec![("query_id", "AAGPK..."), ("user", user_json)];

        let init_data = generate_init_data(&params, bot_token);
        let result = validate_init_data(&init_data, bot_token);

        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::Unauthorized(msg) => assert_eq!(msg, "Missing auth_date"),
            _ => panic!("Expected Unauthorized error"),
        }
    }
}
