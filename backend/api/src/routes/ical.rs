//! iCalendar format serialization/deserialization
//!
//! Converts between our Event model and iCalendar (RFC 5545) format

use chrono::{DateTime, Utc};
use std::borrow::Cow;
use televent_core::models::{Event, EventAttendee, EventStatus, ParticipationStatus};

use crate::error::ApiError;

/// Convert our Event model to iCalendar format
pub fn event_to_ical(event: &Event, attendees: &[EventAttendee]) -> Result<String, ApiError> {
    let mut buf = String::with_capacity(512);
    event_to_ical_into(event, attendees, &mut buf)?;
    Ok(buf)
}

/// Convert our Event model to iCalendar format, writing to a buffer
///
/// This avoids allocating a new String for the result if a buffer is reused.
pub fn event_to_ical_into(
    event: &Event,
    attendees: &[EventAttendee],
    buf: &mut String,
) -> Result<(), ApiError> {
    let mut writer = FoldedWriter::new(buf);

    writer.write_line("BEGIN:VCALENDAR")?;
    writer.write_line("VERSION:2.0")?;
    writer.write_line("PRODID:-//Televent//Televent//EN")?;
    writer.write_line("CALSCALE:GREGORIAN")?;

    writer.write_line("BEGIN:VEVENT")?;

    // UID
    writer.write_property("UID", &event.uid)?;

    // DTSTAMP (required by RFC 5545, indicates when the object was created)
    writer.write_datetime_property("DTSTAMP", &Utc::now())?;

    // Summary
    writer.write_property("SUMMARY", &event.summary)?;

    // Description
    if let Some(ref description) = event.description {
        writer.write_property("DESCRIPTION", description)?;
    }

    // Location
    if let Some(ref location) = event.location {
        writer.write_property("LOCATION", location)?;
    }

    // Start and end times
    if event.is_all_day {
        // All-day events use DATE format (no time component)
        if let Some(start_date) = event.start_date {
            writer.write_date_property("DTSTART;VALUE=DATE", &start_date)?;
        }
    } else if let (Some(start), Some(end)) = (event.start, event.end) {
        writer.write_datetime_property("DTSTART", &start)?;
        writer.write_datetime_property("DTEND", &end)?;
    }

    // Status
    let status_str = match event.status {
        EventStatus::Confirmed => "CONFIRMED",
        EventStatus::Tentative => "TENTATIVE",
        EventStatus::Cancelled => "CANCELLED",
    };
    // Optimization: Status strings are short and safe
    writer.write_safe_property("STATUS", status_str)?;

    // Attendees
    for attendee in attendees {
        let partstat = match attendee.status {
            ParticipationStatus::NeedsAction => "NEEDS-ACTION",
            ParticipationStatus::Accepted => "ACCEPTED",
            ParticipationStatus::Declined => "DECLINED",
            ParticipationStatus::Tentative => "TENTATIVE",
        };

        let prop_name = format!("ATTENDEE;CN=User;RSVP=TRUE;PARTSTAT={}", partstat);
        let value = format!("mailto:{}", attendee.email);
        writer.write_property(&prop_name, &value)?;
    }

    // Recurrence rule
    if let Some(ref rrule) = event.rrule {
        // RRULE is a structured value, do not escape delimiters
        writer.write_property_no_escape("RRULE", rrule)?;
    }

    // Sequence
    // Optimization: Avoid allocating string for integer
    writer.write_int_property("SEQUENCE", event.version)?;

    // Created
    writer.write_datetime_property("CREATED", &event.created_at)?;

    // Last-Modified
    writer.write_datetime_property("LAST-MODIFIED", &event.updated_at)?;

    writer.write_line("END:VEVENT")?;
    writer.write_line("END:VCALENDAR")?;

    Ok(())
}

struct FoldedWriter<'a> {
    buf: &'a mut String,
}

impl<'a> FoldedWriter<'a> {
    fn new(buf: &'a mut String) -> Self {
        Self { buf }
    }

    fn write_line(&mut self, line: &str) -> Result<(), ApiError> {
        self.buf.push_str(line);
        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_property(&mut self, name: &str, value: &str) -> Result<(), ApiError> {
        self.write_property_impl(name, value, true)
    }

    fn write_property_no_escape(&mut self, name: &str, value: &str) -> Result<(), ApiError> {
        self.write_property_impl(name, value, false)
    }

    fn write_safe_property(&mut self, name: &str, value: &str) -> Result<(), ApiError> {
        // Optimization: Write directly to buffer without escaping or folding checks.
        // Use ONLY for values known to be safe (no control chars) and short enough to fit on a line.
        self.buf.push_str(name);
        self.buf.push(':');
        self.buf.push_str(value);
        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_int_property<T: std::fmt::Display>(
        &mut self,
        name: &str,
        value: T,
    ) -> Result<(), ApiError> {
        // Optimization: Write directly to buffer using write! macro to avoid allocation
        self.buf.push_str(name);
        self.buf.push(':');

        use std::fmt::Write;
        write!(self.buf, "{}", value)
            .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;

        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_datetime_property(
        &mut self,
        name: &str,
        datetime: &DateTime<Utc>,
    ) -> Result<(), ApiError> {
        // Optimization: Write directly to buffer without folding checks for value
        // as we know the value is safe (YYYYMMDDTHHmmssZ = 16 chars)
        // and doesn't contain characters needing escaping.
        // We assume name + 1 + 16 <= 75 chars, which is true for standard props.
        self.buf.push_str(name);
        self.buf.push(':');

        use std::fmt::Write;
        // DelayedFormat implements Display
        write!(self.buf, "{}", datetime.format("%Y%m%dT%H%M%SZ"))
            .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;

        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_date_property(
        &mut self,
        name: &str,
        date: &chrono::NaiveDate,
    ) -> Result<(), ApiError> {
        // Optimization: Write directly to buffer
        self.buf.push_str(name);
        self.buf.push(':');

        use std::fmt::Write;
        write!(self.buf, "{}", date.format("%Y%m%d"))
            .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;

        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_property_impl(
        &mut self,
        name: &str,
        value: &str,
        escape: bool,
    ) -> Result<(), ApiError> {
        self.buf.push_str(name);
        self.buf.push(':');

        // Length of property name + separator
        let mut current_line_len = name.len() + 1;

        for c in value.chars() {
            // Strip CR to prevent CRLF injection in all cases
            if c == '\r' {
                continue;
            }

            // Escape special characters: \ ; , \n
            let replacement = if escape {
                match c {
                    '\\' => Some("\\\\"),
                    ';' => Some("\\;"),
                    ',' => Some("\\,"),
                    '\n' => Some("\\n"),
                    _ => None,
                }
            } else {
                None
            };

            if let Some(s) = replacement {
                for rc in s.chars() {
                    let len = rc.len_utf8();
                    if current_line_len + len > 75 {
                        self.buf.push_str("\r\n "); // Fold: CRLF + space
                        current_line_len = 1;
                    }
                    self.buf.push(rc);
                    current_line_len += len;
                }
            } else {
                let len = c.len_utf8();
                if current_line_len + len > 75 {
                    self.buf.push_str("\r\n "); // Fold: CRLF + space
                    current_line_len = 1;
                }
                self.buf.push(c);
                current_line_len += len;
            }
        }
        self.buf.push_str("\r\n");
        Ok(())
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
        Cow<'_, str>,         // uid
        Cow<'_, str>,         // summary
        Option<Cow<'_, str>>, // description
        Option<Cow<'_, str>>, // location
        DateTime<Utc>,        // start
        DateTime<Utc>,        // end
        bool,                 // is_all_day
        Option<Cow<'_, str>>, // rrule
        EventStatus,          // status
        Cow<'_, str>,         // timezone
    ),
    ApiError,
> {
    // Simple line-by-line parser for iCalendar
    // Note: Does not support full RFC 5545 unfolding (continuation lines) perfectly,
    // but handles basic properties.
    // For robust parsing, we should use a proper parser library if needed, but this suffices for our internal use.

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
    let mut timezone = Cow::Borrowed("UTC");

    // Rust borrow checker note:
    // We cannot create a borrow of `line_cow` (local) and return it.
    // We MUST extract ownership or re-borrow from `ical_str`.
    //
    // The previous error `cannot return value referencing local variable line_cow` persists
    // because `line_cow` (even if variant `Borrowed`) is still a local variable binding.
    //
    // However, if `line_cow` is `Borrowed(&'a str)`, we can copy that `&'a str`!
    // `&'a str` implements Copy.
    //
    // So:
    // match line_cow {
    //   Cow::Borrowed(s) => { ... use `s` directly ... }
    // }
    //
    // Inside that branch, `s` is `&'a str`.
    // We can return `Cow::Borrowed(s)`.
    //
    // Let's ensure we use `s` and slices of `s`.

    for line_cow in UnfoldingIter::new(ical_str) {
        match line_cow {
            Cow::Borrowed(line_str) => {
                // line_str has lifetime 'a. It is Copy.
                let line = line_str.trim(); // line is &'a str

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

                if let Some((key, value)) = line.split_once(':') {
                    // key and value are &'a str
                    let (prop_name, params) = if let Some((name, params_str)) = key.split_once(';')
                    {
                        (name, Some(params_str))
                    } else {
                        (key, None)
                    };

                    match prop_name {
                        "UID" => uid = Some(Cow::Borrowed(value)),
                        "SUMMARY" => summary = Some(unescape_text(value)),
                        "DESCRIPTION" => description = Some(unescape_text(value)),
                        "LOCATION" => location = Some(unescape_text(value)),
                        "DTSTART" => {
                            is_all_day = params
                                .map(|p| p.contains("VALUE=DATE") && !p.contains("VALUE=DATE-TIME"))
                                .unwrap_or(false);
                            if let Some(params_str) = params
                                && let Some(tzid_start) = params_str.find("TZID=")
                            {
                                let tz_part = &params_str[tzid_start + 5..];
                                let tz_end = tz_part.find(';').unwrap_or(tz_part.len());
                                timezone = Cow::Borrowed(&params_str[tzid_start + 5..][..tz_end]); // Explicit slice reborrow
                            }
                            dtstart = Some(value.to_string());
                        }
                        "DTEND" => dtend = Some(value.to_string()),
                        "RRULE" => {
                            if value.contains('\r') || value.contains('\n') {
                                return Err(ApiError::BadRequest(
                                    "RRULE cannot contain control characters".to_string(),
                                ));
                            }
                            rrule = Some(Cow::Borrowed(value));
                        }
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
            Cow::Owned(ref line_string) => {
                // line_string is &String (local)
                let line = line_string.trim(); // line is &str (local)

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

                if let Some((key, value)) = line.split_once(':') {
                    let (prop_name, params) = if let Some((name, params_str)) = key.split_once(';')
                    {
                        (name, Some(params_str))
                    } else {
                        (key, None)
                    };

                    match prop_name {
                        "UID" => uid = Some(Cow::Owned(value.to_string())),
                        "SUMMARY" => summary = Some(Cow::Owned(unescape_text(value).into_owned())),
                        "DESCRIPTION" => {
                            description = Some(Cow::Owned(unescape_text(value).into_owned()))
                        }
                        "LOCATION" => {
                            location = Some(Cow::Owned(unescape_text(value).into_owned()))
                        }
                        "DTSTART" => {
                            is_all_day = params
                                .map(|p| p.contains("VALUE=DATE") && !p.contains("VALUE=DATE-TIME"))
                                .unwrap_or(false);
                            if let Some(params_str) = params
                                && let Some(tzid_start) = params_str.find("TZID=")
                            {
                                let tz_part = &params_str[tzid_start + 5..];
                                let tz_end = tz_part.find(';').unwrap_or(tz_part.len());
                                timezone = Cow::Owned(tz_part[..tz_end].to_string());
                            }
                            dtstart = Some(value.to_string());
                        }
                        "DTEND" => dtend = Some(value.to_string()),
                        "RRULE" => {
                            if value.contains('\r') || value.contains('\n') {
                                return Err(ApiError::BadRequest("Invalid RRULE".to_string()));
                            }
                            rrule = Some(Cow::Owned(value.to_string()));
                        }
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
        }
    }

    // Validate required fields
    let uid = uid.ok_or_else(|| ApiError::BadRequest("UID is required".to_string()))?;
    let summary = summary.unwrap_or(Cow::Borrowed("Untitled Event"));
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

/// Iterator that unfolds iCalendar lines (handles continuation lines)
///
/// Yields `Cow<str>` to avoid allocation for non-folded lines.
struct UnfoldingIter<'a> {
    lines: std::iter::Peekable<std::str::Lines<'a>>,
}

impl<'a> UnfoldingIter<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            lines: input.lines().peekable(),
        }
    }
}

impl<'a> Iterator for UnfoldingIter<'a> {
    type Item = std::borrow::Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let first = self.lines.next()?;

        // Check if next line is a continuation (starts with space or tab)
        let is_folded = self
            .lines
            .peek()
            .is_some_and(|l| l.starts_with(' ') || l.starts_with('\t'));

        if !is_folded {
            return Some(std::borrow::Cow::Borrowed(first));
        }

        // If folded, we must allocate to join lines
        let mut folded = String::from(first);
        while let Some(next_line) = self.lines.peek() {
            if next_line.starts_with(' ') || next_line.starts_with('\t') {
                let line = self.lines.next().unwrap();
                // RFC 5545: remove the CRLF (already gone via lines()) and the first whitespace char
                folded.push_str(&line[1..]);
            } else {
                break;
            }
        }
        Some(std::borrow::Cow::Owned(folded))
    }
}

/// Unescape iCalendar text
fn unescape_text(s: &str) -> Cow<'_, str> {
    let bytes = s.as_bytes();

    // Fast path: Find first character that needs escaping (\ or \r)
    // Both are ASCII chars, so we can scan bytes safely as they cannot be part of multi-byte UTF-8 sequences.
    let mut first_special = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\\' || b == b'\r' {
            first_special = Some(i);
            break;
        }
    }

    match first_special {
        None => Cow::Borrowed(s),
        Some(i) => {
            let mut result = String::with_capacity(s.len());
            // Bulk copy safe prefix
            result.push_str(&s[..i]);

            let mut chars = s[i..].chars();
            while let Some(c) = chars.next() {
                // Strip CR to prevent injection (lines() handles CRLF, but not CR alone)
                if c == '\r' {
                    continue;
                }

                if c == '\\' {
                    match chars.next() {
                        Some('n') | Some('N') => result.push('\n'),
                        Some('\\') => result.push('\\'),
                        Some(';') => result.push(';'),
                        Some(',') => result.push(','),
                        Some(other) => {
                            result.push('\\');
                            result.push(other);
                        }
                        None => result.push('\\'), // Trailing backslash
                    }
                } else {
                    result.push(c);
                }
            }
            Cow::Owned(result)
        }
    }
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
        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

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

        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        assert!(ical.contains("BEGIN:VEVENT"));
        assert!(ical.contains("UID:test-event-123"));
        // All-day events should have DATE value
        assert!(ical.contains("DTSTART;VALUE=DATE:20240101"));
    }

    #[test]
    fn test_event_to_ical_with_rrule() {
        let mut event = create_test_event();
        event.rrule = Some("FREQ=DAILY;COUNT=10".to_string());

        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        assert!(ical.contains("RRULE:FREQ=DAILY;COUNT=10"));
    }

    #[test]
    fn test_event_to_ical_statuses() {
        let mut event = create_test_event();

        let attendees = vec![];
        event.status = EventStatus::Confirmed;
        assert!(
            event_to_ical(&event, &attendees)
                .unwrap()
                .contains("STATUS:CONFIRMED")
        );

        event.status = EventStatus::Tentative;
        assert!(
            event_to_ical(&event, &attendees)
                .unwrap()
                .contains("STATUS:TENTATIVE")
        );

        event.status = EventStatus::Cancelled;
        assert!(
            event_to_ical(&event, &attendees)
                .unwrap()
                .contains("STATUS:CANCELLED")
        );
    }

    #[test]
    fn test_event_to_ical_with_attendees() {
        let event = create_test_event();
        let attendees = vec![
            EventAttendee {
                event_id: event.id,
                email: "test@example.com".to_string(),
                user_id: None,
                role: televent_core::models::AttendeeRole::Attendee,
                status: ParticipationStatus::Accepted,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            EventAttendee {
                event_id: event.id,
                email: "decliner@example.com".to_string(),
                user_id: None,
                role: televent_core::models::AttendeeRole::Attendee,
                status: ParticipationStatus::Declined,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let ical = event_to_ical(&event, &attendees).unwrap();

        assert!(
            ical.contains("ATTENDEE;CN=User;RSVP=TRUE;PARTSTAT=ACCEPTED:mailto:test@example.com")
        );
        assert!(
            ical.contains(
                "ATTENDEE;CN=User;RSVP=TRUE;PARTSTAT=DECLINED:mailto:decliner@example.com"
            )
        );
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
        assert_eq!(description, Some(Cow::Borrowed("Test Description")));
        assert_eq!(location, Some(Cow::Borrowed("Test Location")));
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

        assert_eq!(rrule, Some(Cow::Borrowed("FREQ=WEEKLY;BYDAY=MO")));
    }

    #[test]
    fn test_ical_roundtrip() {
        let event = create_test_event();
        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        // Parse it back
        let (uid, summary, description, location, _, _, _, _, status, _) =
            ical_to_event_data(&ical).unwrap();

        assert_eq!(uid, event.uid);
        assert_eq!(summary, event.summary);
        assert_eq!(description, event.description.map(Cow::Owned));
        assert_eq!(location, event.location.map(Cow::Owned));
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
        let attendees = vec![];
        let mut buf = String::new();
        event_to_ical_into(&event, &attendees, &mut buf).unwrap();

        assert!(buf.contains("BEGIN:VCALENDAR"));
        assert!(buf.contains("UID:test-event-123"));
    }

    #[test]
    fn test_folding_and_unfolding() {
        let mut event = create_test_event();
        event.summary = "This is a very long summary that should definitely be folded because it exceeds the seventy-five octet limit imposed by the iCalendar specification (RFC 5545).".to_string();

        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        // Check if it contains CRLF + space
        assert!(ical.contains("\r\n "));

        // Parse it back
        let (_, summary, _, _, _, _, _, _, _, _) = ical_to_event_data(&ical).unwrap();

        assert_eq!(summary, event.summary);
    }

    #[test]
    fn test_unescape_text_edge_cases() {
        // Simple case
        assert_eq!(unescape_text("test"), "test");
        // Escaped chars
        assert_eq!(unescape_text("foo\\;bar"), "foo;bar");
        assert_eq!(unescape_text("foo\\,bar"), "foo,bar");
        assert_eq!(unescape_text("foo\\nbar"), "foo\nbar");
        assert_eq!(unescape_text("foo\\\\bar"), "foo\\bar");
        // Mixed
        assert_eq!(unescape_text("a\\;b\\,c\\nd\\\\e"), "a;b,c\nd\\e");
        // Malformed escape (trailing backslash)
        assert_eq!(unescape_text("foo\\"), "foo\\");
        // Unknown escape
        assert_eq!(unescape_text("foo\\x"), "foo\\x");
        // Tricky case: escaped backslash followed by n (should be literal \n, not newline)
        // Input string literal for testing needs careful escaping.
        // "foo\\\\nbar" in source code is string "foo\\nbar".
        // unescape_text("foo\\nbar") -> "foo\nbar" (newline)
        // unescape_text("foo\\\\nbar") -> "foo\\nbar" (literal \ followed by n)
        assert_eq!(unescape_text("foo\\\\nbar"), "foo\\nbar");
    }

    #[test]
    fn test_event_to_ical_rrule_injection() {
        let mut event = create_test_event();
        // Inject a malicious property via RRULE
        // Note: RRULE validation happens at API boundary, so this tests that the serializer itself is vulnerable
        event.rrule = Some("FREQ=DAILY\r\nATTENDEE:MAILTO:evil@example.com".to_string());

        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        // Check if the injected property appears on its own line
        // The serializer does not escape RRULE, but we now strip CR to prevent CRLF injection
        assert!(!ical.contains("\r\nATTENDEE"));
        // Since we strip CR, it becomes just LF, which is not a valid line break in ical
        assert!(ical.contains("RRULE:FREQ=DAILY\nATTENDEE:MAILTO:evil@example.com"));
    }

    #[test]
    fn test_event_to_ical_cr_injection() {
        let mut event = create_test_event();
        // Inject a malicious property via SUMMARY with just \r (since \n is escaped)
        // If the serializer passes \r through, it might form a newline in lenient parsers
        event.summary = "Hello\rATTENDEE:evil@example.com".to_string();

        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        // Assert that the output does NOT contain a raw CR followed by ATTENDEE
        // We expect the CR to be stripped or escaped
        assert!(
            !ical.contains("\rATTENDEE"),
            "Output contained raw CR injection: {}",
            ical
        );

        // Also ensure it didn't just escape it to \r (literal backslash r) which is not valid but safe
        // Ideally it should be "HelloATTENDEE..." (stripped) or "Hello ATTENDEE..." (replaced)
        // Note: valid ical contains \r\n as line endings, so we can't assert no \r at all.
        // We verify that the injected property name is not preceded by a newline.

        // The output should be "SUMMARY:HelloATTENDEE:..." (stripped)
        assert!(
            ical.contains("SUMMARY:HelloATTENDEE"),
            "CR should be stripped"
        );
    }
}

#[test]
fn test_ical_to_event_data_rrule_injection_prevention() {
    let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:repro\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=DAILY\rATTENDEE:EVIL\r\nEND:VEVENT\r\nEND:VCALENDAR";

    let result = ical_to_event_data(ical);
    assert!(result.is_err());
    match result {
        Err(ApiError::BadRequest(msg)) => {
            assert_eq!(msg, "Invalid RRULE")
        }
        _ => panic!("Expected BadRequest error"),
    }
}

#[test]
fn test_ical_to_event_data_summary_sanitization() {
    let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:repro\r\nDTSTART:20240101T100000Z\r\nSUMMARY:Bad\rSummary\r\nEND:VEVENT\r\nEND:VCALENDAR";

    let (_, summary, _, _, _, _, _, _, _, _) = ical_to_event_data(ical).unwrap();

    // Should be sanitized (stripped CR)
    assert_eq!(summary, "BadSummary");
}
