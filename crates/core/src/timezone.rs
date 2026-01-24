//! Timezone handling utilities
//!
//! Provides functions for parsing and converting timezones safely.

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::error::{CalendarError, CalendarResult};

/// Parse an IANA timezone string (e.g., "Asia/Singapore", "Europe/London")
///
/// # Examples
///
/// ```
/// use televent_core::timezone::parse_timezone;
///
/// let tz = parse_timezone("America/New_York").unwrap();
/// assert_eq!(tz.name(), "America/New_York");
/// ```
pub fn parse_timezone(tz_str: &str) -> CalendarResult<Tz> {
    tz_str
        .parse::<Tz>()
        .map_err(|_| CalendarError::InvalidTimezone(tz_str.to_string()))
}

/// Convert UTC time to a specific timezone
///
/// # Examples
///
/// ```
/// use chrono::Utc;
/// use televent_core::timezone::{parse_timezone, to_timezone};
///
/// let utc_time = Utc::now();
/// let tz = parse_timezone("Asia/Singapore").unwrap();
/// let local_time = to_timezone(&utc_time, &tz);
/// ```
pub fn to_timezone<Tz2: TimeZone>(utc_time: &DateTime<Utc>, tz: &Tz2) -> DateTime<Tz2> {
    utc_time.with_timezone(tz)
}

/// Convert a timezone-aware time to UTC
///
/// # Examples
///
/// ```
/// use chrono::{Utc, TimeZone};
/// use televent_core::timezone::{parse_timezone, to_utc};
///
/// let tz = parse_timezone("Asia/Singapore").unwrap();
/// let local_time = tz.with_ymd_and_hms(2026, 1, 18, 12, 0, 0).unwrap();
/// let utc_time = to_utc(&local_time);
/// ```
pub fn to_utc<Tz2: TimeZone>(time: &DateTime<Tz2>) -> DateTime<Utc> {
    time.with_timezone(&Utc)
}

/// Validate that a timezone string is valid
///
/// Returns `Ok(())` if valid, `Err` otherwise
pub fn validate_timezone(tz_str: &str) -> CalendarResult<()> {
    parse_timezone(tz_str)?;
    Ok(())
}

/// Get the default timezone (UTC)
pub fn default_timezone() -> Tz {
    Tz::UTC
}

/// A validated IANA timezone
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Timezone(String);

impl Timezone {
    /// Create a new Timezone if valid
    pub fn new(tz: &str) -> CalendarResult<Self> {
        validate_timezone(tz)?;
        Ok(Self(tz.to_string()))
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner string
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Timezone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Timelike};

    #[test]
    fn test_parse_timezone_valid() {
        let tz = parse_timezone("America/New_York");
        assert!(tz.is_ok());
        assert_eq!(tz.unwrap().name(), "America/New_York");
    }

    #[test]
    fn test_parse_timezone_singapore() {
        let tz = parse_timezone("Asia/Singapore");
        assert!(tz.is_ok());
        assert_eq!(tz.unwrap().name(), "Asia/Singapore");
    }

    #[test]
    fn test_parse_timezone_invalid() {
        let tz = parse_timezone("Invalid/Timezone");
        assert!(tz.is_err());
        match tz {
            Err(CalendarError::InvalidTimezone(s)) => {
                assert_eq!(s, "Invalid/Timezone");
            }
            _ => panic!("Expected InvalidTimezone error"),
        }
    }

    #[test]
    fn test_utc_to_singapore_conversion() {
        let singapore_tz = parse_timezone("Asia/Singapore").unwrap();

        // Create a UTC time: 2026-01-18 04:00:00 UTC
        let utc_time = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 1, 18)
                .unwrap()
                .and_hms_opt(4, 0, 0)
                .unwrap(),
        );

        // Convert to Singapore time (UTC+8)
        let singapore_time = to_timezone(&utc_time, &singapore_tz);

        // Should be 12:00:00 in Singapore
        assert_eq!(singapore_time.hour(), 12);
        assert_eq!(singapore_time.minute(), 0);
    }

    #[test]
    fn test_singapore_to_utc_conversion() {
        let singapore_tz = parse_timezone("Asia/Singapore").unwrap();

        // Create Singapore time: 2026-01-18 12:00:00 SGT
        let singapore_time = singapore_tz
            .with_ymd_and_hms(2026, 1, 18, 12, 0, 0)
            .unwrap();

        // Convert to UTC (should be 04:00:00)
        let utc_time = to_utc(&singapore_time);

        assert_eq!(utc_time.hour(), 4);
        assert_eq!(utc_time.minute(), 0);
    }

    #[test]
    fn test_validate_timezone() {
        assert!(validate_timezone("Europe/London").is_ok());
        assert!(validate_timezone("America/Los_Angeles").is_ok());
        assert!(validate_timezone("Invalid/Zone").is_err());
    }

    #[test]
    fn test_default_timezone() {
        let tz = default_timezone();
        assert_eq!(tz.name(), "UTC");
    }
}
