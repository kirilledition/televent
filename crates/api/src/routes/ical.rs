//! iCalendar format serialization/deserialization
//!
//! Converts between our Event model and iCalendar (RFC 5545) format

use chrono::{DateTime, Utc};
use televent_core::models::{Event, EventStatus};

use crate::error::ApiError;

/// Convert our Event model to iCalendar format
pub fn event_to_ical(event: &Event) -> Result<String, ApiError> {
    let mut buf = String::with_capacity(512);
    event_to_ical_into(event, &mut buf)?;
    Ok(buf)
}

struct FoldedWriter<'a> {
    buf: &'a mut String,
    current_line_len: usize,
}

impl<'a> FoldedWriter<'a> {
    fn new(buf: &'a mut String) -> Self {
        Self {
            buf,
            current_line_len: 0,
        }
    }

    fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
        }
    }

    fn write_char(&mut self, c: char) {
        let len = c.len_utf8();
        if self.current_line_len + len > 75 {
            self.buf.push_str("\r\n ");
            self.current_line_len = 1 + len;
        } else {
            self.current_line_len += len;
        }
        self.buf.push(c);
    }

    fn end_line(&mut self) {
        self.buf.push_str("\r\n");
        self.current_line_len = 0;
    }
}

/// Convert our Event model to iCalendar format, writing to a buffer
///
/// This avoids allocating a new String for the result if a buffer is reused.
/// optimized to write directly to buffer without intermediate objects.
pub fn event_to_ical_into(event: &Event, buf: &mut String) -> Result<(), ApiError> {
    let mut writer = FoldedWriter::new(buf);

    writer.write_str("BEGIN:VCALENDAR");
    writer.end_line();
    writer.write_str("VERSION:2.0");
    writer.end_line();
    writer.write_str("PRODID:-//Televent//Televent//EN");
    writer.end_line();

    writer.write_str("BEGIN:VEVENT");
    writer.end_line();

    // UID
    writer.write_str("UID:");
    write_escaped(&mut writer, &event.uid);
    writer.end_line();

    // SUMMARY
    writer.write_str("SUMMARY:");
    write_escaped(&mut writer, &event.summary);
    writer.end_line();

    // DESCRIPTION
    if let Some(ref desc) = event.description {
        writer.write_str("DESCRIPTION:");
        write_escaped(&mut writer, desc);
        writer.end_line();
    }

    // LOCATION
    if let Some(ref loc) = event.location {
        writer.write_str("LOCATION:");
        write_escaped(&mut writer, loc);
        writer.end_line();
    }

    // DTSTART / DTEND
    if event.is_all_day {
         if let Some(start_date) = event.start_date {
             writer.write_str("DTSTART;VALUE=DATE:");
             write_date(&mut writer, &start_date);
             writer.end_line();
         }
    } else if let (Some(start), Some(end)) = (event.start, event.end) {
        writer.write_str("DTSTART:");
        write_datetime(&mut writer, &start);
        writer.end_line();

        writer.write_str("DTEND:");
        write_datetime(&mut writer, &end);
        writer.end_line();
    }

    // STATUS
    writer.write_str("STATUS:");
    writer.write_str(match event.status {
        EventStatus::Confirmed => "CONFIRMED",
        EventStatus::Tentative => "TENTATIVE",
        EventStatus::Cancelled => "CANCELLED",
    });
    writer.end_line();

    // RRULE
    if let Some(ref rrule) = event.rrule {
        writer.write_str("RRULE:");
        writer.write_str(rrule);
        writer.end_line();
    }

    // SEQUENCE
    writer.write_str("SEQUENCE:");
    writer.write_str(&event.version.to_string());
    writer.end_line();

    // DTSTAMP (Created)
    writer.write_str("DTSTAMP:");
    write_datetime(&mut writer, &event.created_at);
    writer.end_line();

    // LAST-MODIFIED
    writer.write_str("LAST-MODIFIED:");
    write_datetime(&mut writer, &event.updated_at);
    writer.end_line();

    writer.write_str("END:VEVENT");
    writer.end_line();
    writer.write_str("END:VCALENDAR");
    writer.end_line();

    Ok(())
}

fn write_escaped(writer: &mut FoldedWriter, s: &str) {
    for c in s.chars() {
        match c {
            '\\' => writer.write_str("\\\\"),
            ';' => writer.write_str("\\;"),
            ',' => writer.write_str("\\,"),
            '\n' => writer.write_str("\\n"),
            '\r' => {}, // Ignore carriage returns
            _ => writer.write_char(c),
        }
    }
}

fn write_datetime(writer: &mut FoldedWriter, dt: &DateTime<Utc>) {
    writer.write_str(&dt.format("%Y%m%dT%H%M%SZ").to_string());
}

fn write_date(writer: &mut FoldedWriter, d: &chrono::NaiveDate) {
    writer.write_str(&d.format("%Y%m%d").to_string());
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
    // Handles line unfolding manually
    let mut unfolded_lines = Vec::new();
    let mut current_line = String::new();

    for line in ical_str.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            // Continuation line
            if !current_line.is_empty() {
                // Skip the leading space/tab
                current_line.push_str(&line[1..]);
            }
        } else {
            // New line
            if !current_line.is_empty() {
                unfolded_lines.push(current_line);
            }
            current_line = line.to_string();
        }
    }
    if !current_line.is_empty() {
        unfolded_lines.push(current_line);
    }

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

    for line in unfolded_lines {
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
    fn test_escape_text() {
        let mut buf = String::new();
        let mut writer = FoldedWriter::new(&mut buf);
        write_escaped(&mut writer, "Text with , and ; and \\ and \n newline");
        assert_eq!(buf, "Text with \\, and \\; and \\\\ and \\n newline");
    }

    #[test]
    fn test_line_folding() {
        let mut event = create_test_event();
        // Create a long description (> 75 chars)
        let long_desc = "This is a very long description that should be folded because it exceeds the 75 character limit defined by RFC 5545.";
        event.description = Some(long_desc.to_string());

        let ical = event_to_ical(&event).unwrap();

        // Verify folding
        // Note: exact split depends on property name length "DESCRIPTION:" (12 chars)
        // 75 - 12 = 63 chars allowed on first line.
        // "This is a very long description that should be folded because i" (63 chars)
        // Next line starts with space.

        assert!(ical.contains("\r\n "));

        // Ensure data is preserved (ical_to_event_data handles unfolding now)
        let (_, _, description, _, _, _, _, _, _, _) = ical_to_event_data(&ical).unwrap();
        assert_eq!(description, Some(long_desc.to_string()));
    }
}
