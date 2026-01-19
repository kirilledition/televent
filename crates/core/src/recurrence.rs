//! Recurrence rule handling and validation
//!
//! Provides basic RRULE validation for RFC 5545 recurrence rules.
//!
//! TODO: Full recurrence expansion implementation pending.
//! The `rrule` crate API requires further integration work to properly
//! expand recurring events. For now, we store RRULE strings and can validate them.
//!
//! Future implementation will:
//! - Expand RRULE into individual event instances
//! - Handle EXDATE (exception dates)
//! - Support complex recurrence patterns
//! - Query recurring events efficiently

use crate::error::CalendarError;

/// Parse RRULE string and validate basic format
///
/// Returns an error if the RRULE is obviously malformed
///
/// Note: This is a basic validation. Full RRULE parsing with the rrule crate
/// requires additional type conversions and will be implemented in a future update.
pub fn validate_rrule(rrule_str: &str) -> Result<(), CalendarError> {
    // Basic validation: check for FREQ parameter which is required
    if !rrule_str.contains("FREQ=") {
        return Err(CalendarError::InvalidRRule(
            "RRULE must contain FREQ parameter".to_string(),
        ));
    }

    // Check for valid FREQ values
    let valid_freqs = ["DAILY", "WEEKLY", "MONTHLY", "YEARLY"];
    let has_valid_freq = valid_freqs
        .iter()
        .any(|freq| rrule_str.contains(&format!("FREQ={}", freq)));

    if !has_valid_freq {
        return Err(CalendarError::InvalidRRule(
            "RRULE FREQ must be one of: DAILY, WEEKLY, MONTHLY, YEARLY".to_string(),
        ));
    }

    Ok(())
}

/// Placeholder for future recurrence expansion functionality
///
/// This will be implemented with proper rrule crate integration
pub fn expand_rrule(
    _rrule_str: &str,
    _dtstart: chrono::DateTime<chrono::Utc>,
    _range_start: chrono::DateTime<chrono::Utc>,
    _range_end: chrono::DateTime<chrono::Utc>,
    _max_occurrences: usize,
) -> Result<Vec<chrono::DateTime<chrono::Utc>>, CalendarError> {
    // TODO: Implement with rrule crate
    Err(CalendarError::InvalidRRule(
        "Recurrence expansion not yet implemented".to_string(),
    ))
}

/// Placeholder for next occurrences calculation
///
/// This will be implemented with proper rrule crate integration
pub fn next_occurrences(
    _rrule_str: &str,
    _dtstart: chrono::DateTime<chrono::Utc>,
    _count: usize,
) -> Result<Vec<chrono::DateTime<chrono::Utc>>, CalendarError> {
    // TODO: Implement with rrule crate
    Err(CalendarError::InvalidRRule(
        "Next occurrences calculation not yet implemented".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rrule_valid() {
        assert!(validate_rrule("FREQ=DAILY;COUNT=5").is_ok());
        assert!(validate_rrule("FREQ=WEEKLY;BYDAY=MO,FR").is_ok());
        assert!(validate_rrule("FREQ=MONTHLY;BYMONTHDAY=15").is_ok());
        assert!(validate_rrule("FREQ=YEARLY").is_ok());
    }

    #[test]
    fn test_validate_rrule_missing_freq() {
        assert!(validate_rrule("COUNT=5").is_err());
        assert!(validate_rrule("BYDAY=MO").is_err());
    }

    #[test]
    fn test_validate_rrule_invalid_freq() {
        assert!(validate_rrule("FREQ=INVALID").is_err());
        assert!(validate_rrule("FREQ=HOURLY").is_err()); // Not in our supported list
    }

    #[test]
    fn test_expand_rrule_not_implemented() {
        use chrono::{TimeZone, Utc};
        let dtstart = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 1, 5, 0, 0, 0).unwrap();

        let result = expand_rrule("FREQ=DAILY;COUNT=10", dtstart, range_start, range_end, 100);
        assert!(result.is_err());
    }
}
