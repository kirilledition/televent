//! iCalendar format serialization/deserialization
//!
//! Converts between application calendar data and iCalendar (RFC 5545) format

use chrono::{DateTime, Utc};
use ical::parser::ical::component::IcalEvent;
use televent_domain::{EventStatus, EventTiming, ParticipationStatus};

use crate::ApplicationError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcalEventRender {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub timing: EventTiming,
    pub status: EventStatus,
    pub rrule: Option<String>,
    pub sequence: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcalAttendeeRender {
    pub email: String,
    pub status: ParticipationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcalCalendarEventRender {
    pub event: IcalEventRender,
    pub attendees: Vec<IcalAttendeeRender>,
}

/// Convert application event data to iCalendar format.
pub fn event_to_ical(
    event: &IcalEventRender,
    attendees: &[IcalAttendeeRender],
) -> Result<String, ApplicationError> {
    let mut buf = String::with_capacity(512);
    event_to_ical_into(event, attendees, &mut buf)?;
    Ok(buf)
}

/// Convert our Event model to iCalendar format, writing to a buffer
///
/// This avoids allocating a new String for the result if a buffer is reused.
pub fn event_to_ical_into(
    event: &IcalEventRender,
    attendees: &[IcalAttendeeRender],
    buf: &mut String,
) -> Result<(), ApplicationError> {
    let mut writer = FoldedWriter::new(buf);
    write_calendar_header(&mut writer, None, None)?;
    write_vevent(&mut writer, event, attendees)?;
    writer.write_line("END:VCALENDAR")?;

    Ok(())
}

/// Convert multiple events to a single iCalendar export.
pub fn calendar_to_ical(
    events: &[IcalCalendarEventRender],
    calendar_name: Option<&str>,
    calendar_description: Option<&str>,
) -> Result<String, ApplicationError> {
    let mut buf = String::with_capacity(events.len() * 512 + 256);
    calendar_to_ical_into(events, calendar_name, calendar_description, &mut buf)?;
    Ok(buf)
}

pub fn calendar_to_ical_into(
    events: &[IcalCalendarEventRender],
    calendar_name: Option<&str>,
    calendar_description: Option<&str>,
    buf: &mut String,
) -> Result<(), ApplicationError> {
    let mut writer = FoldedWriter::new(buf);
    write_calendar_header(&mut writer, calendar_name, calendar_description)?;

    for item in events {
        write_vevent(&mut writer, &item.event, &item.attendees)?;
    }

    writer.write_line("END:VCALENDAR")?;

    Ok(())
}

fn write_calendar_header(
    writer: &mut FoldedWriter<'_>,
    calendar_name: Option<&str>,
    calendar_description: Option<&str>,
) -> Result<(), ApplicationError> {
    writer.write_line("BEGIN:VCALENDAR")?;
    writer.write_line("VERSION:2.0")?;
    writer.write_line("PRODID:-//Televent//Televent//EN")?;
    writer.write_line("CALSCALE:GREGORIAN")?;
    if let Some(name) = calendar_name {
        writer.write_property("X-WR-CALNAME", name)?;
    }
    if let Some(description) = calendar_description {
        writer.write_property("X-WR-CALDESC", description)?;
    }

    Ok(())
}

fn write_vevent(
    writer: &mut FoldedWriter<'_>,
    event: &IcalEventRender,
    attendees: &[IcalAttendeeRender],
) -> Result<(), ApplicationError> {
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
    match &event.timing {
        EventTiming::AllDay {
            start_date,
            end_date,
        } => {
            // All-day events use DATE format (no time component)
            writer.write_date_property("DTSTART;VALUE=DATE", start_date)?;
            writer.write_date_property("DTEND;VALUE=DATE", end_date)?;
        }
        EventTiming::Timed { start, end, .. } => {
            writer.write_datetime_property("DTSTART", start)?;
            writer.write_datetime_property("DTEND", end)?;
        }
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
    writer.write_int_property("SEQUENCE", event.sequence)?;

    // Created
    writer.write_datetime_property("CREATED", &event.created_at)?;

    // Last-Modified
    writer.write_datetime_property("LAST-MODIFIED", &event.updated_at)?;

    writer.write_line("END:VEVENT")?;

    Ok(())
}

struct FoldedWriter<'a> {
    buf: &'a mut String,
}

impl<'a> FoldedWriter<'a> {
    fn new(buf: &'a mut String) -> Self {
        Self { buf }
    }

    fn write_line(&mut self, line: &str) -> Result<(), ApplicationError> {
        self.buf.push_str(line);
        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_property(&mut self, name: &str, value: &str) -> Result<(), ApplicationError> {
        self.write_property_impl(name, value, true)
    }

    fn write_property_no_escape(
        &mut self,
        name: &str,
        value: &str,
    ) -> Result<(), ApplicationError> {
        self.write_property_impl(name, value, false)
    }

    fn write_safe_property(&mut self, name: &str, value: &str) -> Result<(), ApplicationError> {
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
    ) -> Result<(), ApplicationError> {
        // Optimization: Write directly to buffer using write! macro to avoid allocation
        self.buf.push_str(name);
        self.buf.push(':');

        use std::fmt::Write;
        write!(self.buf, "{}", value)
            .map_err(|e| ApplicationError::Internal(format!("Format error: {}", e)))?;

        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_datetime_property(
        &mut self,
        name: &str,
        datetime: &DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        // Optimization: Write directly to buffer without folding checks for value
        // as we know the value is safe (YYYYMMDDTHHmmssZ = 16 chars)
        // and doesn't contain characters needing escaping.
        // We assume name + 1 + 16 <= 75 chars, which is true for standard props.
        self.buf.push_str(name);
        self.buf.push(':');

        use std::fmt::Write;
        // DelayedFormat implements Display
        write!(self.buf, "{}", datetime.format("%Y%m%dT%H%M%SZ"))
            .map_err(|e| ApplicationError::Internal(format!("Format error: {}", e)))?;

        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_date_property(
        &mut self,
        name: &str,
        date: &chrono::NaiveDate,
    ) -> Result<(), ApplicationError> {
        // Optimization: Write directly to buffer
        self.buf.push_str(name);
        self.buf.push(':');

        use std::fmt::Write;
        write!(self.buf, "{}", date.format("%Y%m%d"))
            .map_err(|e| ApplicationError::Internal(format!("Format error: {}", e)))?;

        self.buf.push_str("\r\n");
        Ok(())
    }

    fn write_property_impl(
        &mut self,
        name: &str,
        value: &str,
        escape: bool,
    ) -> Result<(), ApplicationError> {
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

/// Parse iCalendar format into event data using ical crate
///
/// Returns (uid, summary, description, location, start, end, is_all_day, rrule, status, timezone)
#[allow(clippy::type_complexity)]
pub fn ical_to_event_data(
    event: &IcalEvent,
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
    ApplicationError,
> {
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

    for prop in &event.properties {
        let value = if let Some(ref v) = prop.value {
            v.as_str()
        } else {
            continue;
        };

        match prop.name.as_str() {
            "UID" => uid = Some(value.to_string()),
            "SUMMARY" => summary = Some(unescape_text(value)),
            "DESCRIPTION" => description = Some(unescape_text(value)),
            "LOCATION" => location = Some(unescape_text(value)),
            "DTSTART" => {
                // Check if this is an all-day event
                // params is Option<Vec<(String, Vec<String>)>>
                if let Some(ref params) = prop.params {
                    for (key, values) in params {
                        if key == "VALUE" && values.iter().any(|v| v == "DATE") {
                            is_all_day = true;
                        }
                        if key == "TZID"
                            && let Some(tzid) = values.first()
                        {
                            timezone = tzid.clone();
                        }
                    }
                }
                dtstart = Some(value.to_string());
            }
            "DTEND" => {
                dtend = Some(value.to_string());
            }
            "RRULE" => {
                if value.contains('\r') || value.contains('\n') {
                    return Err(ApplicationError::BadRequest(
                        "RRULE cannot contain control characters".to_string(),
                    ));
                }
                rrule = Some(value.to_string());
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

    // Validate required fields
    let uid = uid.ok_or_else(|| ApplicationError::BadRequest("UID is required".to_string()))?;
    let summary = summary.unwrap_or_else(|| "Untitled Event".to_string());
    let dtstart_str =
        dtstart.ok_or_else(|| ApplicationError::BadRequest("DTSTART is required".to_string()))?;

    // Parse datetimes
    let start = parse_datetime(&dtstart_str, is_all_day)?;
    let end = if let Some(dtend_str) = dtend {
        parse_datetime(&dtend_str, is_all_day)?
    } else if is_all_day {
        start + chrono::Duration::days(1)
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

/// Unescape iCalendar text
fn unescape_text(s: &str) -> String {
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
        None => s.to_string(),
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
            result
        }
    }
}

/// Parse a datetime string, handling both DATE and DATE-TIME formats
fn parse_datetime(value: &str, is_all_day: bool) -> Result<DateTime<Utc>, ApplicationError> {
    if is_all_day {
        // DATE format: YYYYMMDD
        let date = chrono::NaiveDate::parse_from_str(value, "%Y%m%d")
            .map_err(|e| ApplicationError::BadRequest(format!("Invalid DATE format: {}", e)))?;
        let datetime = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| ApplicationError::BadRequest("Invalid time components".to_string()))?;
        Ok(datetime.and_utc())
    } else {
        // DATE-TIME format: YYYYMMDDTHHmmssZ or YYYYMMDDTHHmmss
        if value.ends_with('Z') {
            let dt = chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S"))
                .map_err(|e| {
                    ApplicationError::BadRequest(format!("Invalid DATE-TIME format: {}", e))
                })?;
            Ok(dt.and_utc())
        } else {
            let dt =
                chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").map_err(|e| {
                    ApplicationError::BadRequest(format!("Invalid DATE-TIME format: {}", e))
                })?;
            Ok(dt.and_utc())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ical::parser::ical::component::IcalEvent;
    use ical::property::Property;

    // Helper to parse ICS string to IcalEvent
    fn parse_ics(ics: &str) -> IcalEvent {
        let parser = ical::IcalParser::new(std::io::Cursor::new(ics));
        let calendar = parser
            .into_iter()
            .next()
            .expect("No calendar")
            .expect("Parse error");
        calendar.events.into_iter().next().expect("No event")
    }

    fn create_test_event() -> IcalEventRender {
        let now = Utc::now();
        IcalEventRender {
            uid: "test-event-123".to_string(),
            summary: "Test Event".to_string(),
            description: Some("Test Description".to_string()),
            location: Some("Test Location".to_string()),
            timing: EventTiming::Timed {
                start: now,
                end: now + chrono::Duration::hours(1),
                timezone: televent_domain::Timezone::utc(),
            },
            rrule: None,
            status: EventStatus::Confirmed,
            sequence: 1,
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
        event.timing = EventTiming::AllDay {
            start_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        };

        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        assert!(ical.contains("BEGIN:VEVENT"));
        assert!(ical.contains("UID:test-event-123"));
        // All-day events should have DATE value
        assert!(ical.contains("DTSTART;VALUE=DATE:20240101"));
        assert!(ical.contains("DTEND;VALUE=DATE:20240102"));
    }

    #[test]
    fn test_calendar_to_ical_multiple_events() {
        let timed_event = create_test_event();
        let mut all_day_event = create_test_event();
        all_day_event.uid = "all-day-event".to_string();
        all_day_event.summary = "All Day Event".to_string();
        all_day_event.timing = EventTiming::AllDay {
            start_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        };

        let ical = calendar_to_ical(
            &[
                IcalCalendarEventRender {
                    event: timed_event,
                    attendees: vec![],
                },
                IcalCalendarEventRender {
                    event: all_day_event,
                    attendees: vec![],
                },
            ],
            Some("Televent Calendar"),
            Some("Exported from Televent Telegram Bot"),
        )
        .unwrap();

        assert_eq!(ical.matches("BEGIN:VCALENDAR").count(), 1);
        assert_eq!(ical.matches("BEGIN:VEVENT").count(), 2);
        assert!(ical.contains("X-WR-CALNAME:Televent Calendar"));
        assert!(ical.contains("X-WR-CALDESC:Exported from Televent Telegram Bot"));
        assert!(ical.contains("SUMMARY:All Day Event"));
        assert!(ical.contains("DTSTART;VALUE=DATE:20240101"));
        assert!(ical.contains("DTEND;VALUE=DATE:20240103"));
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
            IcalAttendeeRender {
                email: "test@example.com".to_string(),
                status: ParticipationStatus::Accepted,
            },
            IcalAttendeeRender {
                email: "decliner@example.com".to_string(),
                status: ParticipationStatus::Declined,
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
        let ical_str = r#"BEGIN:VCALENDAR
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

        let event = parse_ics(ical_str);
        let (uid, summary, description, location, start, end, is_all_day, rrule, status, timezone) =
            ical_to_event_data(&event).unwrap();

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
        let ical_str = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:minimal-event
DTSTART:20240101T100000Z
END:VEVENT
END:VCALENDAR"#;

        let event = parse_ics(ical_str);
        let (uid, summary, _, _, _, _, _, _, _, _) = ical_to_event_data(&event).unwrap();

        assert_eq!(uid, "minimal-event");
        assert_eq!(summary, "Untitled Event"); // Default summary
    }

    #[test]
    fn test_ical_to_event_data_all_day() {
        let ical_str = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:all-day-event
SUMMARY:All Day Event
DTSTART;VALUE=DATE:20240101
DTEND;VALUE=DATE:20240102
END:VEVENT
END:VCALENDAR"#;

        let event = parse_ics(ical_str);
        let (_, _, _, _, start, end, is_all_day, _, _, _) = ical_to_event_data(&event).unwrap();

        assert!(is_all_day);
        assert_eq!(start.format("%Y%m%d").to_string(), "20240101");
        assert_eq!(end.format("%Y%m%d").to_string(), "20240102");
    }

    #[test]
    fn test_ical_to_event_data_all_day_defaults_to_one_day() {
        let ical_str = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:all-day-event-no-end
SUMMARY:All Day Event
DTSTART;VALUE=DATE:20240101
END:VEVENT
END:VCALENDAR"#;

        let event = parse_ics(ical_str);
        let (_, _, _, _, start, end, is_all_day, _, _, _) = ical_to_event_data(&event).unwrap();

        assert!(is_all_day);
        assert_eq!(start.format("%Y%m%d").to_string(), "20240101");
        assert_eq!(end.format("%Y%m%d").to_string(), "20240102");
    }

    #[test]
    fn test_ical_to_event_data_with_rrule() {
        let ical_str = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:recurring-event
SUMMARY:Weekly Meeting
DTSTART:20240101T100000Z
DTEND:20240101T110000Z
RRULE:FREQ=WEEKLY;BYDAY=MO
END:VEVENT
END:VCALENDAR"#;

        let event = parse_ics(ical_str);
        let (_, _, _, _, _, _, _, rrule, _, _) = ical_to_event_data(&event).unwrap();

        assert_eq!(rrule, Some("FREQ=WEEKLY;BYDAY=MO".to_string()));
    }

    #[test]
    fn test_ical_roundtrip() {
        let event = create_test_event();
        let attendees = vec![];
        let ical_str = event_to_ical(&event, &attendees).unwrap();

        // Parse it back
        let ical_event = parse_ics(&ical_str);
        let (uid, summary, description, location, _, _, _, _, status, _) =
            ical_to_event_data(&ical_event).unwrap();

        assert_eq!(uid, event.uid);
        assert_eq!(summary, event.summary);
        assert_eq!(description, event.description);
        assert_eq!(location, event.location);
        assert_eq!(status, event.status);
    }

    #[test]
    fn test_ical_to_event_data_with_timezone() {
        let ical_str = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:tz-event
SUMMARY:Timezone Event
DTSTART;TZID=America/New_York:20240101T100000
DTEND;TZID=America/New_York:20240101T110000
END:VEVENT
END:VCALENDAR"#;

        let event = parse_ics(ical_str);
        let (_, _, _, _, _, _, _, _, _, timezone) = ical_to_event_data(&event).unwrap();

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
        let ical_str = event_to_ical(&event, &attendees).unwrap();

        // Check if it contains CRLF + space
        assert!(ical_str.contains("\r\n "));

        // Parse it back
        let ical_event = parse_ics(&ical_str);
        let (_, summary, _, _, _, _, _, _, _, _) = ical_to_event_data(&ical_event).unwrap();

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
        event.rrule = Some("FREQ=DAILY\r\nATTENDEE:MAILTO:evil@example.com".to_string());

        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        assert!(!ical.contains("\r\nATTENDEE"));
        assert!(ical.contains("RRULE:FREQ=DAILY\nATTENDEE:MAILTO:evil@example.com"));
    }

    #[test]
    fn test_event_to_ical_cr_injection() {
        let mut event = create_test_event();
        // Inject a malicious property via SUMMARY with just \r (since \n is escaped)
        event.summary = "Hello\rATTENDEE:evil@example.com".to_string();

        let attendees = vec![];
        let ical = event_to_ical(&event, &attendees).unwrap();

        assert!(
            !ical.contains("\rATTENDEE"),
            "Output contained raw CR injection: {}",
            ical
        );
        assert!(
            ical.contains("SUMMARY:HelloATTENDEE"),
            "CR should be stripped"
        );
    }

    #[test]
    fn test_ical_to_event_data_rrule_injection_prevention() {
        // Construct IcalEvent manually with malicious RRULE
        let event = IcalEvent {
            properties: vec![
                Property {
                    name: "UID".to_string(),
                    value: Some("repro".to_string()),
                    params: None,
                },
                Property {
                    name: "DTSTART".to_string(),
                    value: Some("20240101T100000Z".to_string()),
                    params: None,
                },
                Property {
                    name: "RRULE".to_string(),
                    value: Some("FREQ=DAILY\rATTENDEE:EVIL".to_string()),
                    params: None,
                },
            ],
            alarms: vec![],
        };

        let result = ical_to_event_data(&event);
        assert!(result.is_err());
        match result {
            Err(ApplicationError::BadRequest(msg)) => {
                assert_eq!(msg, "RRULE cannot contain control characters")
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[test]
    fn test_ical_to_event_data_summary_sanitization() {
        // The ical crate generally doesn't escape text values in .value, but it might handle line unfolding.
        // Our unescape_text function should also handle stripping CR if it ends up there.
        // Let's manually construct an event with CR in summary to test our sanitization.
        let event = IcalEvent {
            properties: vec![
                Property {
                    name: "UID".to_string(),
                    value: Some("repro".to_string()),
                    params: None,
                },
                Property {
                    name: "DTSTART".to_string(),
                    value: Some("20240101T100000Z".to_string()),
                    params: None,
                },
                Property {
                    name: "SUMMARY".to_string(),
                    value: Some("Bad\rSummary".to_string()),
                    params: None,
                },
            ],
            alarms: vec![],
        };

        let (_, summary, _, _, _, _, _, _, _, _) = ical_to_event_data(&event).unwrap();

        // Should be sanitized (stripped CR)
        assert_eq!(summary, "BadSummary");
    }
}
