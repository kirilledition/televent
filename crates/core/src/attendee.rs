//! Attendee email utilities for the Interceptor pattern
//!
//! This module provides helper functions for generating and parsing internal
//! email addresses in the format `tg_<telegram_id>@televent.internal`.
//!
//! The Interceptor pattern routes these internal emails to Telegram notifications
//! instead of SMTP, avoiding paid email service dependencies during MVP phase.

use thiserror::Error;

/// Internal email domain for Televent users
pub const INTERNAL_DOMAIN: &str = "televent.internal";

/// Errors that can occur when processing attendee emails
#[derive(Error, Debug)]
pub enum AttendeeError {
    #[error("Invalid email format: {0}")]
    InvalidFormat(String),

    #[error("Invalid telegram_id: {0}")]
    InvalidTelegramId(String),
}

/// Generate an internal email address from a Telegram ID
///
/// # Examples
///
/// ```
/// use televent_core::attendee::generate_internal_email;
///
/// let email = generate_internal_email(123456789);
/// assert_eq!(email, "tg_123456789@televent.internal");
/// ```
pub fn generate_internal_email(telegram_id: i64) -> String {
    format!("tg_{}@{}", telegram_id, INTERNAL_DOMAIN)
}

/// Extract Telegram ID from an internal email address
///
/// Returns `Ok(Some(telegram_id))` for valid internal emails,
/// `Ok(None)` for external emails (not @televent.internal),
/// or `Err` if the format is invalid.
///
/// # Examples
///
/// ```
/// use televent_core::attendee::extract_telegram_id;
///
/// // Internal email
/// let result = extract_telegram_id("tg_123456789@televent.internal").unwrap();
/// assert_eq!(result, Some(123456789));
///
/// // External email
/// let result = extract_telegram_id("user@gmail.com").unwrap();
/// assert_eq!(result, None);
/// ```
pub fn extract_telegram_id(email: &str) -> Result<Option<i64>, AttendeeError> {
    // Check if it's an internal email
    if !email.ends_with(&format!("@{}", INTERNAL_DOMAIN)) {
        return Ok(None);
    }

    // Extract local part
    let local_part = email
        .split('@')
        .next()
        .ok_or_else(|| AttendeeError::InvalidFormat(email.to_string()))?;

    // Verify tg_ prefix
    if !local_part.starts_with("tg_") {
        return Err(AttendeeError::InvalidFormat(format!(
            "Expected 'tg_' prefix, got: {}",
            local_part
        )));
    }

    // Parse telegram_id
    let id_str = &local_part[3..];
    let telegram_id = id_str
        .parse::<i64>()
        .map_err(|_| AttendeeError::InvalidTelegramId(id_str.to_string()))?;

    Ok(Some(telegram_id))
}

/// Check if an email address is an internal Televent email
///
/// # Examples
///
/// ```
/// use televent_core::attendee::is_internal_email;
///
/// assert!(is_internal_email("tg_123456789@televent.internal"));
/// assert!(!is_internal_email("user@gmail.com"));
/// ```
pub fn is_internal_email(email: &str) -> bool {
    email.ends_with(&format!("@{}", INTERNAL_DOMAIN))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_internal_email() {
        assert_eq!(
            generate_internal_email(123456789),
            "tg_123456789@televent.internal"
        );
        assert_eq!(generate_internal_email(999), "tg_999@televent.internal");
    }

    #[test]
    fn test_extract_telegram_id_valid_internal() {
        let result = extract_telegram_id("tg_123456789@televent.internal").unwrap();
        assert_eq!(result, Some(123456789));

        let result = extract_telegram_id("tg_999@televent.internal").unwrap();
        assert_eq!(result, Some(999));
    }

    #[test]
    fn test_extract_telegram_id_external_email() {
        let result = extract_telegram_id("user@gmail.com").unwrap();
        assert_eq!(result, None);

        let result = extract_telegram_id("john@outlook.com").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_telegram_id_invalid_format() {
        // Missing tg_ prefix
        let result = extract_telegram_id("123456789@televent.internal");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AttendeeError::InvalidFormat(_)
        ));

        // Invalid telegram_id (not a number)
        let result = extract_telegram_id("tg_abc@televent.internal");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AttendeeError::InvalidTelegramId(_)
        ));

        // Empty telegram_id
        let result = extract_telegram_id("tg_@televent.internal");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AttendeeError::InvalidTelegramId(_)
        ));
    }

    #[test]
    fn test_is_internal_email() {
        // Valid internal emails
        assert!(is_internal_email("tg_123456789@televent.internal"));
        assert!(is_internal_email("tg_999@televent.internal"));
        assert!(is_internal_email("anything@televent.internal"));

        // External emails
        assert!(!is_internal_email("user@gmail.com"));
        assert!(!is_internal_email("john@outlook.com"));
        assert!(!is_internal_email("test@example.com"));
        assert!(!is_internal_email("tg_123@wrongdomain.com"));
    }

    #[test]
    fn test_roundtrip_conversion() {
        // Generate internal email and extract telegram_id back
        let original_id = 123456789i64;
        let email = generate_internal_email(original_id);
        let extracted_id = extract_telegram_id(&email).unwrap();
        assert_eq!(extracted_id, Some(original_id));
    }

    #[test]
    fn test_is_internal_matches_extract() {
        // is_internal_email and extract_telegram_id should be consistent
        let internal = "tg_123@televent.internal";
        let external = "user@gmail.com";

        assert_eq!(
            is_internal_email(internal),
            extract_telegram_id(internal).unwrap().is_some()
        );
        assert_eq!(
            is_internal_email(external),
            extract_telegram_id(external).unwrap().is_some()
        );
    }
}
