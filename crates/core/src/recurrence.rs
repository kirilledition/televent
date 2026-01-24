//! Recurrence rule handling and validation
//!
//! Provides RRULE validation and expansion using the `rrule` crate.

use crate::error::CalendarError;
use chrono::{DateTime, Utc};
use rrule::{RRuleError, RRuleSet, Tz};

/// Parse RRULE string and validate format
pub fn validate_rrule(rrule_str: &str) -> Result<(), CalendarError> {
    // To validate the RRULE part, we need to provide a dummy DTSTART
    // because the rrule crate requires it for strict parsing.
    let dummy_start = "DTSTART:20240101T000000Z\nRRULE:";
    let full_str = format!("{}{}", dummy_start, rrule_str);

    full_str
        .parse::<RRuleSet>()
        .map_err(|e: RRuleError| CalendarError::InvalidRRule(e.to_string()))?;

    Ok(())
}

/// Expand recurrence rule into occurrence dates
pub fn expand_rrule(
    rrule_str: &str,
    dtstart: DateTime<Utc>,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
    max_occurrences: usize,
) -> Result<Vec<DateTime<Utc>>, CalendarError> {
    // Construct the full RRULE string with DTSTART
    // Note: We format DTSTART in UTC
    let dtstart_str = dtstart.format("%Y%m%dT%H%M%SZ").to_string();
    let full_str = format!("DTSTART:{}\nRRULE:{}", dtstart_str, rrule_str);

    // Parse the RRULE
    let rrule_set: RRuleSet = full_str
        .parse()
        .map_err(|e: RRuleError| CalendarError::InvalidRRule(e.to_string()))?;

    // Generate occurrences
    // rrule crate returns generic DateTime, we expect it to be in UTC because input was UTC
    // We filter by range and limit
    let occurrences = rrule_set
        .into_iter()
        .skip_while(|d: &DateTime<Tz>| *d < range_start)
        .take_while(|d: &DateTime<Tz>| *d <= range_end)
        .take(max_occurrences)
        .map(|d: DateTime<Tz>| d.with_timezone(&Utc))
        .collect();

    Ok(occurrences)
}

/// Calculate next occurrences from a given time
pub fn next_occurrences(
    rrule_str: &str,
    dtstart: DateTime<Utc>,
    count: usize,
) -> Result<Vec<DateTime<Utc>>, CalendarError> {
    let dtstart_str = dtstart.format("%Y%m%dT%H%M%SZ").to_string();
    let full_str = format!("DTSTART:{}\nRRULE:{}", dtstart_str, rrule_str);

    let rrule_set: RRuleSet = full_str
        .parse()
        .map_err(|e: RRuleError| CalendarError::InvalidRRule(e.to_string()))?;

    // We want the next occurrences starting from NOW (or just the first 'count' ones?)
    // "next occurrences" usually implies future ones relative to "now",
    // but the signature doesn't take "now".
    // Assuming it means "the first `count` occurrences of this rule".
    // If it meant "next from now", we would need a 'from' parameter.
    // Given the signature, let's return the first `count` occurrences.

    let occurrences = rrule_set
        .all(count as u16)
        .dates
        .into_iter()
        .map(|d: DateTime<Tz>| d.with_timezone(&Utc))
        .collect();

    Ok(occurrences)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone};

    #[test]
    fn test_validate_rrule_valid() {
        assert!(validate_rrule("FREQ=DAILY;COUNT=5").is_ok());
        assert!(validate_rrule("FREQ=WEEKLY;BYDAY=MO,FR").is_ok());
    }

    #[test]
    fn test_validate_rrule_invalid() {
        assert!(validate_rrule("INVALID=TRUE").is_err());
    }

    #[test]
    fn test_expand_rrule_daily() {
        let dtstart = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let range_start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 1, 5, 0, 0, 0).unwrap();

        // Daily for 3 days
        let occurrences =
            expand_rrule("FREQ=DAILY;COUNT=3", dtstart, range_start, range_end, 10).unwrap();

        assert_eq!(occurrences.len(), 3);
        assert_eq!(occurrences[0], dtstart);
        assert_eq!(occurrences[1], dtstart + chrono::Duration::days(1));
        assert_eq!(occurrences[2], dtstart + chrono::Duration::days(2));
    }

    #[test]
    fn test_expand_rrule_outside_range() {
        let dtstart = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        // Range starts after the first few occurrences
        let range_start = Utc.with_ymd_and_hms(2026, 1, 4, 0, 0, 0).unwrap();
        let range_end = Utc.with_ymd_and_hms(2026, 1, 6, 0, 0, 0).unwrap();

        let occurrences =
            expand_rrule("FREQ=DAILY;COUNT=10", dtstart, range_start, range_end, 10).unwrap();

        // 2026-01-01 (skip)
        // 2026-01-02 (skip)
        // 2026-01-03 (skip)
        // 2026-01-04 (keep)
        // 2026-01-05 (keep)
        // 2026-01-06 (keep - 10:00 > 00:00? Wait, take_while check)

        // dtstart is 10:00.
        // Jan 4 10:00 >= Jan 4 00:00 -> Keep
        // Jan 5 10:00 <= Jan 6 00:00 -> False (Jan 5 10:00 < Jan 6 00:00 is true)
        // Jan 6 10:00 <= Jan 6 00:00 -> False

        // So we expect Jan 4 and Jan 5.
        // Let's verify expectations.
        // Jan 4 10:00
        // Jan 5 10:00
        // Jan 6 10:00 (is it <= Jan 6 00:00? No)

        assert_eq!(occurrences[0].day(), 4);
        assert_eq!(occurrences[1].day(), 5);
    }

    #[test]
    fn test_next_occurrences() {
        let dtstart = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        // Daily for 3 days
        let occurrences = next_occurrences("FREQ=DAILY", dtstart, 3).unwrap();

        assert_eq!(occurrences.len(), 3);
        assert_eq!(occurrences[0], dtstart);
        assert_eq!(occurrences[1], dtstart + chrono::Duration::days(1));
        assert_eq!(occurrences[2], dtstart + chrono::Duration::days(2));
    }
}
