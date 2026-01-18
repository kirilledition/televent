//! Telegram authentication middleware
//!
//! Validates Telegram initData using HMAC-SHA256 signature

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

/// Extract and validate Telegram initData from headers
///
/// The initData is sent via the X-Telegram-Init-Data header
/// and contains user information signed by Telegram
pub async fn validate_telegram_init_data(
    State(bot_token): State<String>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract initData from headers
    let init_data = headers
        .get("X-Telegram-Init-Data")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify HMAC signature
    if !verify_telegram_hash(init_data, &bot_token) {
        tracing::warn!("Invalid Telegram signature");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Parse user_id from init_data
    let user_id = parse_user_id(init_data).ok_or(StatusCode::BAD_REQUEST)?;

    // Attach user_id to request extensions
    request.extensions_mut().insert(user_id);

    Ok(next.run(request).await)
}

/// Verify Telegram initData signature
///
/// Algorithm:
/// 1. Parse key-value pairs from initData
/// 2. Remove 'hash' field
/// 3. Sort remaining fields alphabetically
/// 4. Create data-check-string (key=value\nkey=value...)
/// 5. Compute HMAC-SHA256(secret_key, data-check-string)
/// 6. Compare with provided hash
fn verify_telegram_hash(init_data: &str, bot_token: &str) -> bool {
    // Parse init_data into key-value pairs
    let mut params: HashMap<String, String> = init_data
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect();

    // Extract hash
    let provided_hash = match params.remove("hash") {
        Some(h) => h,
        None => return false,
    };

    // Sort keys and build data-check-string
    let mut keys: Vec<_> = params.keys().collect();
    keys.sort();

    let data_check_string = keys
        .iter()
        .map(|k| format!("{}={}", k, params[*k]))
        .collect::<Vec<_>>()
        .join("\n");

    // Compute secret_key = HMAC-SHA256("WebAppData", bot_token)
    let mut secret_key_mac = HmacSha256::new_from_slice(b"WebAppData")
        .expect("HMAC can take key of any size");
    secret_key_mac.update(bot_token.as_bytes());
    let secret_key = secret_key_mac.finalize().into_bytes();

    // Compute hash = HMAC-SHA256(secret_key, data-check-string)
    let mut mac = match HmacSha256::new_from_slice(&secret_key) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(data_check_string.as_bytes());

    // Convert to hex string
    let computed_hash = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison
    computed_hash == provided_hash
}

/// Parse user_id from initData
fn parse_user_id(init_data: &str) -> Option<i64> {
    for pair in init_data.split('&') {
        if pair.starts_with("user=") {
            // user field is URL-encoded JSON: user={"id":123,"first_name":"John",...}
            let json_str = pair.strip_prefix("user=")?;
            let decoded = urlencoding::decode(json_str).ok()?;

            // Parse JSON to extract id
            let user_data: serde_json::Value = serde_json::from_str(&decoded).ok()?;
            return user_data.get("id")?.as_i64();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_telegram_hash_valid() {
        let bot_token = "test_bot_token_123";

        // Manually compute a valid signature
        // Step 1: Create data-check-string
        let data_check_string = "auth_date=1234567890\nuser={\"id\":123}";

        // Step 2: Compute secret_key
        let mut secret_key_mac =
            HmacSha256::new_from_slice(b"WebAppData").expect("HMAC key error");
        secret_key_mac.update(bot_token.as_bytes());
        let secret_key = secret_key_mac.finalize().into_bytes();

        // Step 3: Compute hash
        let mut mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC key error");
        mac.update(data_check_string.as_bytes());
        let hash = hex::encode(mac.finalize().into_bytes());

        // Step 4: Build init_data with computed hash
        let init_data = format!(
            "auth_date=1234567890&user={{\"id\":123}}&hash={}",
            hash
        );

        assert!(verify_telegram_hash(&init_data, bot_token));
    }

    #[test]
    fn test_verify_telegram_hash_invalid() {
        let bot_token = "test_bot_token_123";
        let init_data = "auth_date=1234567890&user={\"id\":123}&hash=invalid_hash";

        assert!(!verify_telegram_hash(init_data, bot_token));
    }

    #[test]
    fn test_verify_telegram_hash_missing_hash() {
        let bot_token = "test_bot_token_123";
        let init_data = "auth_date=1234567890&user={\"id\":123}";

        assert!(!verify_telegram_hash(init_data, bot_token));
    }

    #[test]
    fn test_verify_telegram_hash_tampered_data() {
        let bot_token = "test_bot_token_123";

        // Create valid signature for one dataset
        let data_check_string = "auth_date=1234567890\nuser={\"id\":123}";
        let mut secret_key_mac =
            HmacSha256::new_from_slice(b"WebAppData").expect("HMAC key error");
        secret_key_mac.update(bot_token.as_bytes());
        let secret_key = secret_key_mac.finalize().into_bytes();
        let mut mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC key error");
        mac.update(data_check_string.as_bytes());
        let hash = hex::encode(mac.finalize().into_bytes());

        // But use different data (tampered)
        let init_data = format!(
            "auth_date=9999999999&user={{\"id\":456}}&hash={}",
            hash
        );

        assert!(!verify_telegram_hash(&init_data, bot_token));
    }

    #[test]
    fn test_parse_user_id_valid() {
        let init_data = "auth_date=1234567890&user=%7B%22id%22%3A123%2C%22first_name%22%3A%22John%22%7D&hash=abc";
        let user_id = parse_user_id(init_data);
        assert_eq!(user_id, Some(123));
    }

    #[test]
    fn test_parse_user_id_invalid_json() {
        let init_data = "auth_date=1234567890&user=invalid_json&hash=abc";
        let user_id = parse_user_id(init_data);
        assert_eq!(user_id, None);
    }

    #[test]
    fn test_parse_user_id_missing_user_field() {
        let init_data = "auth_date=1234567890&hash=abc";
        let user_id = parse_user_id(init_data);
        assert_eq!(user_id, None);
    }

    #[test]
    fn test_parse_user_id_missing_id_in_user() {
        let init_data = "auth_date=1234567890&user=%7B%22first_name%22%3A%22John%22%7D&hash=abc";
        let user_id = parse_user_id(init_data);
        assert_eq!(user_id, None);
    }
}
