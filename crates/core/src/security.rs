//! Security utilities for Televent
//!
//! This module provides security-related functionality including:
//! - Password hashing with Argon2id
//! - ETag generation for CalDAV
//! - Telegram authentication validation

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Generate an ETag for CalDAV from event data
///
/// ETags must be based on content, not timestamps, to avoid false conflicts
/// with clock skew between client and server.
///
/// # Arguments
/// * `data` - The event data to hash
///
/// # Returns
/// A SHA256 hash as a hex string suitable for use as an HTTP ETag
pub fn generate_etag(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Hash a password using Argon2id
///
/// Uses secure defaults:
/// - Memory: 19456 KiB (19 MiB)
/// - Iterations: 2
/// - Parallelism: 1
/// - Output length: 32 bytes
///
/// # Arguments
/// * `password` - The plaintext password to hash
///
/// # Returns
/// An Argon2id hash string suitable for storage
///
/// # Errors
/// Returns an error if hashing fails
pub fn hash_password(password: &str) -> Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
    
    Ok(hash.to_string())
}

/// Verify a password against an Argon2id hash
///
/// # Arguments
/// * `password` - The plaintext password to verify
/// * `hash` - The Argon2id hash to verify against
///
/// # Returns
/// `Ok(true)` if the password matches, `Ok(false)` if it doesn't
///
/// # Errors
/// Returns an error if the hash is malformed
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("Failed to parse password hash: {}", e))?;
    
    let argon2 = Argon2::default();
    
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Validate Telegram init data using HMAC-SHA256
///
/// Verifies the authenticity of data received from Telegram Login Widget
/// by checking the HMAC signature.
///
/// # Arguments
/// * `init_data` - The init data string from Telegram
/// * `bot_token` - The bot token to use as the secret key
///
/// # Returns
/// `Ok(true)` if the signature is valid, `Ok(false)` otherwise
///
/// # Errors
/// Returns an error if HMAC computation fails
pub fn verify_telegram_init_data(init_data: &str, bot_token: &str) -> Result<bool> {
    // Parse init_data into key-value pairs
    let mut params: Vec<(&str, &str)> = init_data
        .split('&')
        .filter_map(|param| {
            let mut parts = param.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some(key), Some(value)) if key != "hash" => Some((key, value)),
                _ => None,
            }
        })
        .collect();

    // Extract the hash from init_data
    let received_hash = init_data
        .split('&')
        .find_map(|param| {
            if param.starts_with("hash=") {
                param.strip_prefix("hash=")
            } else {
                None
            }
        })
        .context("Hash not found in init_data")?;

    // Sort parameters alphabetically by key
    params.sort_by_key(|(k, _)| *k);

    // Create data check string
    let data_check_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    // Compute secret key from bot token
    let mut secret_key_mac = HmacSha256::new_from_slice(b"WebAppData")
        .map_err(|e| anyhow::anyhow!("Failed to create HMAC: {}", e))?;
    secret_key_mac.update(bot_token.as_bytes());
    let secret_key = secret_key_mac.finalize().into_bytes();

    // Compute hash
    let mut mac = HmacSha256::new_from_slice(&secret_key)
        .map_err(|e| anyhow::anyhow!("Failed to create HMAC: {}", e))?;
    mac.update(data_check_string.as_bytes());
    let computed_hash = hex::encode(mac.finalize().into_bytes());

    Ok(computed_hash == received_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_etag() {
        let data = b"test event data";
        let etag1 = generate_etag(data);
        let etag2 = generate_etag(data);
        
        // Same data should produce same ETag
        assert_eq!(etag1, etag2);
        
        // Different data should produce different ETag
        let etag3 = generate_etag(b"different data");
        assert_ne!(etag1, etag3);
        
        // ETag should be hex string
        assert!(etag1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_password_hashing() {
        let password = "secure_password_123";
        let hash = hash_password(password).unwrap();
        
        // Hash should not be empty
        assert!(!hash.is_empty());
        
        // Hash should start with Argon2id identifier
        assert!(hash.starts_with("$argon2"));
        
        // Verification should succeed with correct password
        assert!(verify_password(password, &hash).unwrap());
        
        // Verification should fail with wrong password
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_password_hashing_different_salts() {
        let password = "test123";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();
        
        // Same password should produce different hashes (different salts)
        assert_ne!(hash1, hash2);
        
        // Both hashes should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }
}
