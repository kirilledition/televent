//! iCalendar format serialization/deserialization
//!
//! Converts between our Event model and iCalendar (RFC 5545) format

use chrono::{DateTime, Utc};
use televent_core::models::{Event, EventStatus};

use crate::error::ApiError;

/// Convert our Event model to iCalendar format
pub fn event_to_ical(event: &Event) -> Result<String, ApiError> {
    let mut buf = String::with_capacity(1024);
    event_to_ical_into(event, &mut buf)?;
    Ok(buf)
}

/// Convert our Event model to iCalendar format, writing to a buffer
///
/// This avoids allocating a new String for the result if a buffer is reused.
pub fn event_to_ical_into(event: &Event, buf: &mut String) -> Result<(), ApiError> {
    let mut writer = FoldedWriter::new(buf);

    // VCALENDAR Header
    writer.write_line("BEGIN:VCALENDAR");
    writer.write_line("VERSION:2.0");
    writer.write_line("PRODID:-//Televent//Televent//EN");
    writer.write_line("CALSCALE:GREGORIAN");

    // VEVENT
    writer.write_line("BEGIN:VEVENT");

    // UID
    writer.write_property("UID", &event.uid);

    // Summary
    writer.write_property("SUMMARY", &event.summary);

    // Description
    if let Some(ref description) = event.description {
        writer.write_property("DESCRIPTION", description);
    }

    // Location
    if let Some(ref location) = event.location {
        writer.write_property("LOCATION", location);
    }

    // Start and End
    if event.is_all_day {
        // All-day events use DATE format (no time component)
        // Matches previous behavior which only used start_date
        if let Some(start_date) = event.start_date {
            writer.write_property("DTSTART;VALUE=DATE", &start_date.format("%Y%m%d").to_string());
        }
    } else if let (Some(start), Some(end)) = (event.start, event.end) {
        // Format: YYYYMMDDTHHmmssZ
        writer.write_property("DTSTART", &start.format("%Y%m%dT%H%M%SZ").to_string());
        writer.write_property("DTEND", &end.format("%Y%m%dT%H%M%SZ").to_string());
    }

    // Status
    let status_str = match event.status {
        EventStatus::Confirmed => "CONFIRMED",
        EventStatus::Tentative => "TENTATIVE",
        EventStatus::Cancelled => "CANCELLED",
    };
    writer.write_property("STATUS", status_str);

    // Recurrence Rule
    if let Some(ref rrule) = event.rrule {
        // RRULE values (RECUR type) should not be text-escaped (e.g. ; should remain ; not \;)
        writer.write_property_no_escape("RRULE", rrule);
    }

    // Sequence
    writer.write_property("SEQUENCE", &event.version.to_string());

    // Created (DTSTAMP) and Last Modified
    // RFC 5545 requires DTSTAMP. We use created_at to match previous behavior.
    writer.write_property(
        "DTSTAMP",
        &event.created_at.format("%Y%m%dT%H%M%SZ").to_string(),
    );
    writer.write_property(
        "LAST-MODIFIED",
        &event.updated_at.format("%Y%m%dT%H%M%SZ").to_string(),
    );

    writer.write_line("END:VEVENT");
    writer.write_line("END:VCALENDAR");

    Ok(())
}

/// Helper to write folded lines for iCalendar (RFC 5545)
struct FoldedWriter<'a> {
    out: &'a mut String,
    line_len: usize,
}

impl<'a> FoldedWriter<'a> {
    fn new(out: &'a mut String) -> Self {
        Self { out, line_len: 0 }
    }

    /// Write a full property line: NAME:VALUE
    fn write_property(&mut self, name: &str, value: &str) {
        self.write_raw(name);
        self.write_char(':');
        self.write_escaped(value);
        self.end_line();
    }

    /// Write a property without escaping value (for non-TEXT types like RECUR)
    fn write_property_no_escape(&mut self, name: &str, value: &str) {
        self.write_raw(name);
        self.write_char(':');
        self.write_raw(value);
        self.end_line();
    }

    /// Write a raw line (no escaping, no folding check on the string itself)
    fn write_line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push_str("\r\n");
        self.line_len = 0;
    }

    fn write_raw(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
        }
    }

    fn write_escaped(&mut self, s: &str) {
        // RFC 5545: TEXT values need escaping for \ , ; and \n
        for c in s.chars() {
            match c {
                '\\' => self.write_raw(r"\\"),
                ';' => self.write_raw(r"\;"),
                ',' => self.write_raw(r"\,"),
                '\n' => self.write_raw(r"\n"),
                _ => self.write_char(c),
            }
        }
    }

    fn write_char(&mut self, c: char) {
        let char_len = c.len_utf8();
        // RFC 5545: Lines SHOULD NOT be longer than 75 octets
        if self.line_len + char_len > 75 {
            self.out.push_str("\r\n "); // Fold with CRLF + Space
            self.line_len = 1; // The space counts as 1 octet
        }
        self.out.push(c);
        self.line_len += char_len;
    }

    fn end_line(&mut self) {
        self.out.push_str("\r\n");
        self.line_len = 0;
    }
}

/// Parse iCalendar format into event data (simple string-based parser)
///
/// Returns (uid, summary, description, location, start, end, is_all_day, rrule, status, timezone)
#[allow(clippy::type_complexity)]
pub fn ical_to_event_data(
    ical_str: &str,
) -> Result<
    (
        String,
        String,
        Option<String>,
        Option<String>,
        DateTime<Utc>,
        DateTime<Utc>,
        bool,
        Option<String>,
        EventStatus,
        String,
    ),
    ApiError,
> {
    // Simple line-by-line parser for iCalendar
    let mut in_vevent = false;
    let mut uid = None;
    let mut summary = None;
    let mut description = None;
    let mut location = None;
    let mut dtstart = None;
    let mut dtend = None;
    let mut is_all_day = false;
    let mut rrule = None;
    let mut status = EventStatus::Confirmed;
    let mut timezone = "UTC".to_string();

    // We need to handle folded lines (unfolding)
    // RFC 5545: "lines ... that begin with a linear white-space ... are considered to be a number of folded lines"
    // To simplify, we can reconstruct unfolded lines first, or handle it in iterator.
    // Given the previous implementation didn't explicitly unfold (it just trimmed!), let's verify if that was correct.
    // Previous:
    // for line in ical_str.lines() {
    //     let line = line.trim();
    // This effectively "unfolds" by removing leading spaces from the line content, BUT it treats it as a new line!
    // RFC folding:
    // Line 1: SUMMARY:This is a long line that is f
    // Line 2:  olded.
    // Unfolded: SUMMARY:This is a long line that is folded.
    // Previous code:
    // Loop 1: "SUMMARY:This is a long line that is f" -> parsed
    // Loop 2: "olded." -> ignored (no :) or maybe broken
    // So previous code was likely BROKEN for folded lines!
    // But since I am only optimizing generation, I should probably leave parsing logic alone unless I want to fix it.
    // The previous parsing logic was:
    // for line in ical_str.lines() {
    //     let line = line.trim();
    //     ...
    //     if let Some((key, value)) = line.split_once(':') ...
    // If a line was folded, the second line starts with space. `trim()` removes it. "olded.". `split_once(':')` fails.
    // So it was definitely broken for folded lines.
    // However, fixing parsing is out of scope for "performance improvement" unless it's necessary for my tests.
    // I will restore previous parsing logic exactly to avoid changing behavior/scope creep.

    for line in ical_str.lines() {
        let line = line.trim();

        if line == "BEGIN:VEVENT" {
            in_vevent = true;
            continue;
        }

        if line == "END:VEVENT" {
            break;
        }

        if !in_vevent {
            continue;
        }

        // Parse property lines
        if let Some((key, value)) = line.split_once(':') {
            let (prop_name, params) = if let Some((name, params_str)) = key.split_once(';') {
                (name, Some(params_str))
            } else {
                (key, None)
            };

            match prop_name {
                "UID" => uid = Some(value.to_string()),
                "SUMMARY" => summary = Some(value.to_string()),
                "DESCRIPTION" => description = Some(value.to_string()),
                "LOCATION" => location = Some(value.to_string()),
                "DTSTART" => {
                    // Check if this is an all-day event
                    is_all_day = params
                        .map(|p| p.contains("VALUE=DATE") && !p.contains("VALUE=DATE-TIME"))
                        .unwrap_or(false);
                    // Extract timezone from TZID parameter if present
                    #[allow(clippy::collapsible_if)]
                    if let Some(params_str) = params {
                        if let Some(tzid_start) = params_str.find("TZID=") {
                            let tz_part = &params_str[tzid_start + 5..];
                            let tz_end = tz_part.find(';').unwrap_or(tz_part.len());
                            timezone = tz_part[..tz_end].to_string();
                        }
                    }
                    dtstart = Some(value.to_string());
                }
                "DTEND" => {
                    dtend = Some(value.to_string());
                }
                "RRULE" => rrule = Some(value.to_string()),
                "STATUS" => {
                    status = match value.to_uppercase().as_str() {
                        "CONFIRMED" => EventStatus::Confirmed,
                        "TENTATIVE" => EventStatus::Tentative,
                        "CANCELLED" => EventStatus::Cancelled,
                        _ => EventStatus::Confirmed,
                    };
                }
                _ => {}
            }
        }
    }

    // Validate required fields
    let uid = uid.ok_or_else(|| ApiError::BadRequest("UID is required".to_string()))?;
    let summary = summary.unwrap_or_else(|| "Untitled Event".to_string());
    let dtstart_str =
        dtstart.ok_or_else(|| ApiError::BadRequest("DTSTART is required".to_string()))?;

    // Parse datetimes
    let start = parse_datetime(&dtstart_str, is_all_day)?;
    let end = if let Some(dtend_str) = dtend {
        parse_datetime(&dtend_str, is_all_day)?
    } else {
        // Default to 1 hour duration
        start + chrono::Duration::hours(1)
    };

    Ok((
        uid,
        summary,
        description,
        location,
        start,
        end,
        is_all_day,
        rrule,
        status,
        timezone,
    ))
}

/// Parse a datetime string, handling both DATE and DATE-TIME formats
fn parse_datetime(value: &str, is_all_day: bool) -> Result<DateTime<Utc>, ApiError> {
    if is_all_day {
        // DATE format: YYYYMMDD
        let date = chrono::NaiveDate::parse_from_str(value, "%Y%m%d")
            .map_err(|e| ApiError::BadRequest(format!("Invalid DATE format: {}", e)))?;
        let datetime = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| ApiError::BadRequest("Invalid time components".to_string()))?;
        Ok(datetime.and_utc())
    } else {
        // DATE-TIME format: YYYYMMDDTHHmmssZ or YYYYMMDDTHHmmss
        if value.ends_with('Z') {
            let dt = chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S"))
                .map_err(|e| ApiError::BadRequest(format!("Invalid DATE-TIME format: {}", e)))?;
            Ok(dt.and_utc())
        } else {
            let dt = chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
                .map_err(|e| ApiError::BadRequest(format!("Invalid DATE-TIME format: {}", e)))?;
            Ok(dt.and_utc())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use televent_core::models::UserId;
    use uuid::Uuid;

    fn create_test_event() -> Event {
        use televent_core::models::Timezone;
        let now = Utc::now();
        Event {
            id: Uuid::new_v4(),
            user_id: UserId::new(123456789),
            uid: "test-event-123".to_string(),
            version: 1,
            etag: "abc123".to_string(),
            summary: "Test Event".to_string(),
            description: Some("Test Description".to_string()),
            location: Some("Test Location".to_string()),
            start: Some(now),
            end: Some(now + chrono::Duration::hours(1)),
            start_date: None,
            end_date: None,
            is_all_day: false,
            rrule: None,
            status: EventStatus::Confirmed,
            timezone: Timezone::default(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_event_to_ical_basic() {
        let event = create_test_event();
        let ical = event_to_ical(&event).unwrap();

        assert!(ical.contains("BEGIN:VCALENDAR"));
        assert!(ical.contains("BEGIN:VEVENT"));
        assert!(ical.contains("UID:test-event-123"));
        assert!(ical.contains("SUMMARY:Test Event"));
        assert!(ical.contains("DESCRIPTION:Test Description"));
        assert!(ical.contains("LOCATION:Test Location"));
        assert!(ical.contains("STATUS:CONFIRMED"));
        assert!(ical.contains("END:VEVENT"));
        assert!(ical.contains("END:VCALENDAR"));
    }

    #[test]
    fn test_event_to_ical_all_day() {
        let mut event = create_test_event();
        event.is_all_day = true;
        event.start_date = Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        event.end_date = Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());

        let ical = event_to_ical(&event).unwrap();

        assert!(ical.contains("BEGIN:VEVENT"));
        assert!(ical.contains("UID:test-event-123"));
        // All-day events should have DATE value
        assert!(ical.contains("DTSTART;VALUE=DATE:20240101"));
    }

    #[test]
    fn test_event_to_ical_with_rrule() {
        let mut event = create_test_event();
        event.rrule = Some("FREQ=DAILY;COUNT=10".to_string());

        let ical = event_to_ical(&event).unwrap();

        assert!(ical.contains("RRULE:FREQ=DAILY;COUNT=10"));
    }

    #[test]
    fn test_event_to_ical_statuses() {
        let mut event = create_test_event();

        event.status = EventStatus::Confirmed;
        assert!(event_to_ical(&event).unwrap().contains("STATUS:CONFIRMED"));

        event.status = EventStatus::Tentative;
        assert!(event_to_ical(&event).unwrap().contains("STATUS:TENTATIVE"));

        event.status = EventStatus::Cancelled;
        assert!(event_to_ical(&event).unwrap().contains("STATUS:CANCELLED"));
    }

    #[test]
    fn test_ical_to_event_data_basic() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test-123
SUMMARY:Test Event
DESCRIPTION:Test Description
LOCATION:Test Location
DTSTART:20240101T100000Z
DTEND:20240101T110000Z
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR"#;

        let (uid, summary, description, location, start, end, is_all_day, rrule, status, timezone) =
            ical_to_event_data(ical).unwrap();

        assert_eq!(uid, "test-123");
        assert_eq!(summary, "Test Event");
        assert_eq!(description, Some("Test Description".to_string()));
        assert_eq!(location, Some("Test Location".to_string()));
        assert!(!is_all_day);
        assert_eq!(rrule, None);
        assert_eq!(status, EventStatus::Confirmed);
        assert_eq!(timezone, "UTC");
        assert!(end > start);
    }

    #[test]
    fn test_ical_to_event_data_minimal() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:minimal-event
DTSTART:20240101T100000Z
END:VEVENT
END:VCALENDAR"#;

        let (uid, summary, _, _, _, _, _, _, _, _) = ical_to_event_data(ical).unwrap();

        assert_eq!(uid, "minimal-event");
        assert_eq!(summary, "Untitled Event"); // Default summary
    }

    #[test]
    fn test_ical_to_event_data_all_day() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:all-day-event
SUMMARY:All Day Event
DTSTART;VALUE=DATE:20240101
DTEND;VALUE=DATE:20240102
END:VEVENT
END:VCALENDAR"#;

        let (_, _, _, _, start, end, is_all_day, _, _, _) = ical_to_event_data(ical).unwrap();

        assert!(is_all_day);
        assert_eq!(start.format("%Y%m%d").to_string(), "20240101");
        assert_eq!(end.format("%Y%m%d").to_string(), "20240102");
    }

    #[test]
    fn test_ical_to_event_data_with_rrule() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:recurring-event
SUMMARY:Weekly Meeting
DTSTART:20240101T100000Z
DTEND:20240101T110000Z
RRULE:FREQ=WEEKLY;BYDAY=MO
END:VEVENT
END:VCALENDAR"#;

        let (_, _, _, _, _, _, _, rrule, _, _) = ical_to_event_data(ical).unwrap();

        assert_eq!(rrule, Some("FREQ=WEEKLY;BYDAY=MO".to_string()));
    }

    #[test]
    fn test_ical_roundtrip() {
        let event = create_test_event();
        let ical = event_to_ical(&event).unwrap();

        // Parse it back
        let (uid, summary, description, location, _, _, _, _, status, _) =
            ical_to_event_data(&ical).unwrap();

        assert_eq!(uid, event.uid);
        assert_eq!(summary, event.summary);
        assert_eq!(description, event.description);
        assert_eq!(location, event.location);
        assert_eq!(status, event.status);
    }

    #[test]
    fn test_ical_to_event_data_with_timezone() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:tz-event
SUMMARY:Timezone Event
DTSTART;TZID=America/New_York:20240101T100000
DTEND;TZID=America/New_York:20240101T110000
END:VEVENT
END:VCALENDAR"#;

        let (_, _, _, _, _, _, _, _, _, timezone) = ical_to_event_data(ical).unwrap();

        assert_eq!(timezone, "America/New_York");
    }

    #[test]
    fn test_event_to_ical_into() {
        let event = create_test_event();
        let mut buf = String::new();
        event_to_ical_into(&event, &mut buf).unwrap();

        assert!(buf.contains("BEGIN:VCALENDAR"));
        assert!(buf.contains("UID:test-event-123"));
    }

    #[test]
    fn test_ical_folding_and_escaping() {
        let mut event = create_test_event();
        // Create a long summary that triggers folding (> 75 octets)
        // "This is a very long summary that should definitely trigger line folding because it is longer than 75 characters."
        event.summary = "This is a very long summary that should definitely trigger line folding because it is longer than 75 characters.".to_string();
        // Description with special characters
        event.description = Some("Line 1\nLine 2; with semicolon, and comma".to_string());

        let ical = event_to_ical(&event).unwrap();

        // Check escaping
        assert!(ical.contains(r"Line 1\nLine 2\; with semicolon\, and comma"));

        // Check folding
        // We can't easily check exact string because folding implementation details (where exactly it splits),
        // but we can check that no line is super long.
        for line in ical.lines() {
            // Lines can be longer than 75 if they contain multi-byte chars?
            // My implementation checks UTF-8 length.
            // But strict octet count is what matters.
            // Also line break is not included.
            assert!(line.len() <= 78); // 75 + CRLF? My code pushes CRLF then space.
            // Wait, my code: if len > 75 { push \r\n (2) + space (1); reset len=1 }
            // So visually it splits.
        }

        // Also check content presence (split across lines)
        let unfolded = ical.replace("\r\n ", "");
        assert!(unfolded.contains(&event.summary));
    }
}
