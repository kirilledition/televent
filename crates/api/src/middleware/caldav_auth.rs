//! CalDAV Basic Authentication middleware
//!
//! Validates device passwords for CalDAV clients using HTTP Basic Auth

use crate::AppState;
use crate::error::ApiError;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use uuid::Uuid;

/// CalDAV Basic Auth middleware
///
/// Expects Authorization header with format: "Basic base64(telegram_id:password)"
/// Verifies password against device_passwords table using Argon2id
pub async fn caldav_basic_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

    // Parse Basic Auth
    let (telegram_id, password) = parse_basic_auth(auth_header)?;

    // Check Cache
    if let Some(user_id) = state.auth_cache.get(&(telegram_id, password.clone())).await {
        request.extensions_mut().insert(user_id);
        return Ok(next.run(request).await);
    }

    // Look up user by telegram_id
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE telegram_id = $1")
        .bind(telegram_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    // Get device passwords for this user
    let device_passwords: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, hashed_password FROM device_passwords WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    // Verify password against each device password
    let mut verified_device_id: Option<Uuid> = None;
    for (device_id, hashed_password) in device_passwords {
        if verify_password(&password, &hashed_password)? {
            verified_device_id = Some(device_id);
            break;
        }
    }

    let device_id = verified_device_id
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    // Update last_used_at timestamp
    sqlx::query("UPDATE device_passwords SET last_used_at = NOW() WHERE id = $1")
        .bind(device_id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::warn!("Failed to update last_used_at: {}", e);
            // Don't fail the request if we can't update the timestamp
        })
        .ok();

    // Cache success
    state
        .auth_cache
        .insert((telegram_id, password), user_id)
        .await;

    // Attach user_id to request extensions
    request.extensions_mut().insert(user_id);

    Ok(next.run(request).await)
}

/// Parse HTTP Basic Auth header
///
/// Expected format: "Basic base64(telegram_id:password)"
/// Returns (telegram_id, password)
fn parse_basic_auth(auth_header: &str) -> Result<(i64, String), ApiError> {
    // Check for "Basic " prefix
    let encoded = auth_header
        .strip_prefix("Basic ")
        .ok_or_else(|| ApiError::Unauthorized("Invalid Authorization header".to_string()))?;

    // Decode base64
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| ApiError::Unauthorized("Invalid base64 encoding".to_string()))?;

    let credentials = String::from_utf8(decoded)
        .map_err(|_| ApiError::Unauthorized("Invalid UTF-8 in credentials".to_string()))?;

    // Split on first colon
    let mut parts = credentials.splitn(2, ':');
    let telegram_id_str = parts
        .next()
        .ok_or_else(|| ApiError::Unauthorized("Missing telegram_id".to_string()))?;
    let password = parts
        .next()
        .ok_or_else(|| ApiError::Unauthorized("Missing password".to_string()))?;

    // Parse telegram_id as i64
    let telegram_id = telegram_id_str
        .parse::<i64>()
        .map_err(|_| ApiError::Unauthorized(
            "Username must be your Telegram ID (numeric). Get it from the /device command in Telegram bot.".to_string()
        ))?;

    Ok((telegram_id, password.to_string()))
}

/// Verify password using Argon2id
fn verify_password(password: &str, hashed_password: &str) -> Result<bool, ApiError> {
    let parsed_hash = PasswordHash::new(hashed_password)
        .map_err(|e| ApiError::Internal(format!("Invalid password hash: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };

    #[test]
    fn test_parse_basic_auth_valid() {
        let credentials = "123456789:my_password";
        let encoded = STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        let result = parse_basic_auth(&auth_header);
        assert!(result.is_ok());

        let (telegram_id, password) = result.unwrap();
        assert_eq!(telegram_id, 123456789);
        assert_eq!(password, "my_password");
    }

    #[test]
    fn test_parse_basic_auth_with_colon_in_password() {
        let credentials = "123456789:pass:word:with:colons";
        let encoded = STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        let result = parse_basic_auth(&auth_header);
        assert!(result.is_ok());

        let (telegram_id, password) = result.unwrap();
        assert_eq!(telegram_id, 123456789);
        assert_eq!(password, "pass:word:with:colons");
    }

    #[test]
    fn test_parse_basic_auth_missing_prefix() {
        let credentials = "123456789:my_password";
        let encoded = STANDARD.encode(credentials.as_bytes());

        let result = parse_basic_auth(&encoded);
        assert!(result.is_err());
        match result {
            Err(ApiError::Unauthorized(msg)) => {
                assert!(msg.contains("Invalid Authorization header"));
            }
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_parse_basic_auth_invalid_base64() {
        let auth_header = "Basic invalid!!!base64";

        let result = parse_basic_auth(auth_header);
        assert!(result.is_err());
        match result {
            Err(ApiError::Unauthorized(msg)) => {
                assert!(msg.contains("Invalid base64"));
            }
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_parse_basic_auth_missing_password() {
        let credentials = "123456789";
        let encoded = STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        let result = parse_basic_auth(&auth_header);
        assert!(result.is_err());
        match result {
            Err(ApiError::Unauthorized(msg)) => {
                assert!(msg.contains("Missing password"));
            }
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_parse_basic_auth_invalid_telegram_id() {
        let credentials = "not_a_number:my_password";
        let encoded = STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        let result = parse_basic_auth(&auth_header);
        assert!(result.is_err());
        match result {
            Err(ApiError::Unauthorized(msg)) => {
                assert!(msg.contains("Invalid telegram_id format"));
            }
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_verify_password_valid() {
        let password = "test_password_123";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let result = verify_password(password, &password_hash);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_password_invalid() {
        let password = "test_password_123";
        let wrong_password = "wrong_password";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let result = verify_password(wrong_password, &password_hash);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let password = "test_password_123";
        let invalid_hash = "not_a_valid_argon2_hash";

        let result = verify_password(password, invalid_hash);
        assert!(result.is_err());
        match result {
            Err(ApiError::Internal(msg)) => {
                assert!(msg.contains("Invalid password hash"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_verify_password_empty_password() {
        let password = "";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let result = verify_password(password, &password_hash);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
