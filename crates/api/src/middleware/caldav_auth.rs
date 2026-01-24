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

/// Login identifier: either a numeric Telegram ID or a username (without @)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoginId {
    TelegramId(i64),
    Username(String),
}

/// CalDAV Basic Auth middleware
///
/// Expects Authorization header with format: "Basic base64(login_id:password)"
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
    let (login_id, password) = parse_basic_auth(auth_header)?;

    // Check Cache
    if let Some(user_id) = state
        .auth_cache
        .get(&(login_id.clone(), password.clone()))
        .await
    {
        request.extensions_mut().insert(user_id);
        return Ok(next.run(request).await);
    }

    // Look up user by login_id
    let user_id: Uuid = match &login_id {
        LoginId::TelegramId(tid) => {
            sqlx::query_scalar("SELECT id FROM users WHERE telegram_id = $1")
                .bind(tid)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| ApiError::Internal(format!("Database error: {e}")))?
                .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?
        }
        LoginId::Username(username) => {
            sqlx::query_scalar("SELECT id FROM users WHERE lower(telegram_username) = lower($1)")
                .bind(username)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| ApiError::Internal(format!("Database error: {e}")))?
                .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?
        }
    };

    // Get device passwords for this user
    // Optimization: Limit to 10 devices to prevent DoS
    // We order by last_used_at to prioritize active devices
    let device_passwords: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, hashed_password FROM device_passwords \
         WHERE user_id = $1 \
         ORDER BY last_used_at DESC NULLS LAST, created_at DESC \
         LIMIT 10",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {e}")))?;

    // Verify password against each device password
    let mut verified_device_id: Option<Uuid> = None;
    for (device_id, hashed_password) in device_passwords {
        // Argon2 verification is expensive, so this loop is the critical path.
        // We limit it to 10 iterations above.
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
    state.auth_cache.insert((login_id, password), user_id).await;

    // Attach user_id to request extensions
    request.extensions_mut().insert(user_id);

    Ok(next.run(request).await)
}

/// Parse HTTP Basic Auth header
///
/// Expected format: "Basic base64(login_id:password)"
/// Returns (login_id, password)
fn parse_basic_auth(auth_header: &str) -> Result<(LoginId, String), ApiError> {
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
    let login_str = parts
        .next()
        .ok_or_else(|| ApiError::Unauthorized("Missing username".to_string()))?;
    let password = parts
        .next()
        .ok_or_else(|| ApiError::Unauthorized("Missing password".to_string()))?;

    // Try to parse as numeric ID, otherwise treat as username
    let login_id = if let Ok(tid) = login_str.parse::<i64>() {
        LoginId::TelegramId(tid)
    } else {
        // Remove @ if present (though CalDAV clients usually don't send it)
        let username = login_str.trim_start_matches('@').to_string();
        if username.is_empty() {
            return Err(ApiError::Unauthorized(
                "Username cannot be empty".to_string(),
            ));
        }
        LoginId::Username(username)
    };

    Ok((login_id, password.to_string()))
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
    fn test_parse_basic_auth_valid_numeric() {
        let credentials = "123456789:my_password";
        let encoded = STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        let result = parse_basic_auth(&auth_header);
        assert!(result.is_ok());

        let (login_id, password) = result.unwrap();
        assert_eq!(login_id, LoginId::TelegramId(123456789));
        assert_eq!(password, "my_password");
    }

    #[test]
    fn test_parse_basic_auth_valid_username() {
        let credentials = "prince:my_password";
        let encoded = STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        let result = parse_basic_auth(&auth_header);
        assert!(result.is_ok());

        let (login_id, password) = result.unwrap();
        assert_eq!(login_id, LoginId::Username("prince".to_string()));
        assert_eq!(password, "my_password");
    }

    #[test]
    fn test_parse_basic_auth_valid_username_with_at() {
        let credentials = "@prince:my_password";
        let encoded = STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        let result = parse_basic_auth(&auth_header);
        assert!(result.is_ok());

        let (login_id, password) = result.unwrap();
        assert_eq!(login_id, LoginId::Username("prince".to_string()));
        assert_eq!(password, "my_password");
    }

    #[test]
    fn test_parse_basic_auth_with_colon_in_password() {
        let credentials = "123456789:pass:word:with:colons";
        let encoded = STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);

        let result = parse_basic_auth(&auth_header);
        assert!(result.is_ok());

        let (login_id, password) = result.unwrap();
        assert_eq!(login_id, LoginId::TelegramId(123456789));
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
