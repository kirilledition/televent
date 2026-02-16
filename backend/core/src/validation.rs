//! Validation constants and utilities
//!
//! Shared validation logic for inputs across REST API and CalDAV.

/// Maximum length for iCalendar UID (RFC 5545 doesn't specify limit, but practical limit is 256)
pub const MAX_UID_LENGTH: usize = 256;

/// Maximum length for event summary/title
pub const MAX_SUMMARY_LENGTH: usize = 256;

/// Maximum length for event description
pub const MAX_DESCRIPTION_LENGTH: usize = 10000;

/// Maximum length for event location
pub const MAX_LOCATION_LENGTH: usize = 1024;

/// Maximum length for RRULE string
pub const MAX_RRULE_LENGTH: usize = 1024;

/// Validate string length
pub fn validate_length(field_name: &str, value: &str, max_len: usize) -> Result<(), String> {
    if value.len() > max_len {
        Err(format!("{} too long (max {})", field_name, max_len))
    } else {
        Ok(())
    }
}

/// Validate that a string contains no control characters (except tab)
///
/// Useful for single-line fields (summary, location, UID) to prevent injection
/// and garbage characters. Allows tab (\t).
pub fn validate_no_control_chars(field_name: &str, value: &str) -> Result<(), String> {
    if value.chars().any(|c| c.is_control() && c != '\t') {
        Err(format!("{} cannot contain control characters", field_name))
    } else {
        Ok(())
    }
}

/// Validate that a string contains no dangerous control characters
///
/// Allows newline (\n), carriage return (\r), and tab (\t), but bans others (null, bell, etc.).
/// Useful for multiline fields (description).
pub fn validate_safe_multiline_text(field_name: &str, value: &str) -> Result<(), String> {
    if value
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        Err(format!("{} cannot contain control characters", field_name))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_length() {
        assert!(validate_length("Test", "short", 10).is_ok());
        assert!(validate_length("Test", "exactlength", 11).is_ok());
        assert!(validate_length("Test", "toolong", 5).is_err());
    }

    #[test]
    fn test_validate_no_control_chars() {
        assert!(validate_no_control_chars("Test", "clean string").is_ok());
        assert!(validate_no_control_chars("Test", "string with\ttab").is_ok());
        assert!(validate_no_control_chars("Test", "dirty\rstring").is_err());
        assert!(validate_no_control_chars("Test", "dirty\nstring").is_err());
        assert!(validate_no_control_chars("Test", "dirty\r\nstring").is_err());
        assert!(validate_no_control_chars("Test", "dirty\0string").is_err());
        assert!(validate_no_control_chars("Test", "dirty\x07string").is_err());
    }

    #[test]
    fn test_validate_safe_multiline_text() {
        assert!(validate_safe_multiline_text("Test", "clean string").is_ok());
        assert!(validate_safe_multiline_text("Test", "multiline\nstring").is_ok());
        assert!(validate_safe_multiline_text("Test", "multiline\r\nstring").is_ok());
        assert!(validate_safe_multiline_text("Test", "string with\ttab").is_ok());
        assert!(validate_safe_multiline_text("Test", "dirty\0string").is_err());
        assert!(validate_safe_multiline_text("Test", "dirty\x07string").is_err());
        assert!(validate_safe_multiline_text("Test", "dirty\x1Fstring").is_err());
    }
}
