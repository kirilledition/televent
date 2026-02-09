//! Attendee email utilities for the Interceptor pattern
//!
//! This module provides helper functions for generating and parsing internal
//! email addresses in the format `tg_<telegram_id>@televent.internal`.
//!
//! The Interceptor pattern routes these internal emails to Telegram notifications
//! instead of SMTP, avoiding paid email service dependencies during MVP phase.

use crate::models::UserId;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Internal email domain for Televent users
pub const INTERNAL_DOMAIN: &str = "televent.internal";

/// Errors that can occur when processing attendee emails
#[derive(Error, Debug, PartialEq)]
pub enum AttendeeError {
    #[error("Invalid email format: {0}")]
    InvalidFormat(String),

    #[error("Invalid telegram_id: {0}")]
    InvalidTelegramId(String),
}

/// Value Object representing an internal Televent email address
///
/// Wraps a UserId to ensure type safety and correct formatting.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternalEmail(UserId);

impl InternalEmail {
    /// Create a new InternalEmail from a UserId
    pub fn new(user_id: UserId) -> Self {
        Self(user_id)
    }

    /// Get the underlying UserId
    pub fn user_id(&self) -> UserId {
        self.0
    }

    /// Get the email address as a string
    pub fn as_str(&self) -> String {
        format!("tg_{}@{}", self.0, INTERNAL_DOMAIN)
    }
}

impl fmt::Display for InternalEmail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for InternalEmail {
    type Err = AttendeeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let user_id =
            parse_internal_email(s).ok_or_else(|| AttendeeError::InvalidFormat(s.to_string()))?;
        Ok(InternalEmail(user_id))
    }
}

/// Generate an internal email address from a User ID
///
/// # Examples
///
/// ```
/// use televent_core::attendee::generate_internal_email;
/// use televent_core::models::UserId;
///
/// let user_id = UserId::new(123456789);
/// let email = generate_internal_email(user_id);
/// assert_eq!(email, "tg_123456789@televent.internal");
/// ```
pub fn generate_internal_email(user_id: UserId) -> String {
    InternalEmail::new(user_id).as_str()
}

/// Parse an internal email address to extract the User ID
///
/// Returns `Some(UserId)` if the email is a valid internal email,
/// `None` otherwise.
///
/// # Examples
///
/// ```
/// use televent_core::attendee::parse_internal_email;
/// use televent_core::models::UserId;
///
/// // Valid internal email
/// let email = "tg_123456789@televent.internal";
/// let user_id = parse_internal_email(email);
/// assert_eq!(user_id, Some(UserId::new(123456789)));
///
/// // Invalid or external email
/// assert_eq!(parse_internal_email("user@gmail.com"), None);
/// assert_eq!(parse_internal_email("tg_abc@televent.internal"), None);
/// ```
pub fn parse_internal_email(email: &str) -> Option<UserId> {
    // Check if it's an internal email
    if !email.ends_with(&format!("@{}", INTERNAL_DOMAIN)) {
        return None;
    }

    // Extract local part
    let local_part = email.split('@').next()?;

    // Verify tg_ prefix
    if !local_part.starts_with("tg_") {
        return None;
    }

    // Parse telegram_id
    let id_str = &local_part[3..];
    match id_str.parse::<i64>() {
        Ok(id) => Some(UserId::new(id)),
        Err(_) => None,
    }
}

/// Extract Telegram ID from an internal email address
///
/// DEPRECATED: Use `parse_internal_email` instead.
///
/// Returns `Ok(Some(telegram_id))` for valid internal emails,
/// `Ok(None)` for external emails (not @televent.internal),
/// or `Err` if the format is invalid but looks like it aimed to be internal.
pub fn extract_telegram_id(email: &str) -> Result<Option<i64>, AttendeeError> {
    // Check if it's an internal email
    if !email.ends_with(&format!("@{}", INTERNAL_DOMAIN)) {
        return Ok(None);
    }

    match parse_internal_email(email) {
        Some(user_id) => Ok(Some(user_id.inner())),
        None => {
            // If parse_internal_email returns None but it ended with the domain,
            // it means the format was invalid (e.g. not tg_ prefix or not a number).
            // We need to reconstruct the specific error for backward compatibility if possible,
            // or just return a generic format error.
            if !email.starts_with("tg_")
                && email.contains('@')
                && !email.split('@').next().unwrap_or("").starts_with("tg_")
            {
                // Trying to match exact error messages from before might be brittle,
                // but let's try to be helpful.
                let local_part = email.split('@').next().unwrap_or("");
                Err(AttendeeError::InvalidFormat(format!(
                    "Expected 'tg_' prefix, got: {}",
                    local_part
                )))
            } else {
                let local_part = email.split('@').next().unwrap_or("");
                if let Some(id_str) = local_part.strip_prefix("tg_") {
                    Err(AttendeeError::InvalidTelegramId(id_str.to_string()))
                } else {
                    Err(AttendeeError::InvalidFormat(email.to_string()))
                }
            }
        }
    }
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
    fn test_internal_email_struct() {
        let user_id = UserId::new(123456789);
        let email = InternalEmail::new(user_id);
        assert_eq!(email.user_id(), user_id);
        assert_eq!(email.as_str(), "tg_123456789@televent.internal");
        assert_eq!(email.to_string(), "tg_123456789@televent.internal");
    }

    #[test]
    fn test_generate_internal_email() {
        let user_id = UserId::new(123456789);
        assert_eq!(
            generate_internal_email(user_id),
            "tg_123456789@televent.internal"
        );
        assert_eq!(
            generate_internal_email(UserId::new(999)),
            "tg_999@televent.internal"
        );
    }

    #[test]
    fn test_parse_internal_email() {
        // Valid
        assert_eq!(
            parse_internal_email("tg_123456789@televent.internal"),
            Some(UserId::new(123456789))
        );

        // Invalid domain
        assert_eq!(parse_internal_email("tg_123@gmail.com"), None);

        // Missing prefix
        assert_eq!(parse_internal_email("123@televent.internal"), None);

        // Invalid ID
        assert_eq!(parse_internal_email("tg_abc@televent.internal"), None);
        assert_eq!(parse_internal_email("tg_@televent.internal"), None);
    }

    #[test]
    fn test_extract_telegram_id_backward_compat() {
        // Valid
        let result = extract_telegram_id("tg_123456789@televent.internal").unwrap();
        assert_eq!(result, Some(123456789));

        // External
        let result = extract_telegram_id("user@gmail.com").unwrap();
        assert_eq!(result, None);

        // Invalid format errors
        let result = extract_telegram_id("123@televent.internal");
        assert!(matches!(result, Err(AttendeeError::InvalidFormat(_))));

        let result = extract_telegram_id("tg_abc@televent.internal");
        assert!(matches!(result, Err(AttendeeError::InvalidTelegramId(_))));
    }

    #[test]
    fn test_is_internal_email() {
        assert!(is_internal_email("tg_123@televent.internal"));
        assert!(!is_internal_email("user@gmail.com"));
    }
}
