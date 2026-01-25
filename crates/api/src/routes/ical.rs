//! iCalendar format serialization/deserialization
//!
//! Converts between our Event model and iCalendar (RFC 5545) format

use chrono::{DateTime, Utc};
use icalendar::{Calendar, Component, Event as IcalEvent, EventLike};
use televent_core::models::{Event, EventStatus};

use crate::error::ApiError;

/// Convert our Event model to iCalendar format
pub fn event_to_ical(event: &Event) -> Result<String, ApiError> {
    let mut ical_event = IcalEvent::new();

    // UID is required and must be stable
    ical_event.uid(&event.uid);

    // Summary (title)
    ical_event.summary(&event.summary);

    // Description
    if let Some(ref description) = event.description {
        ical_event.description(description);
    }

    // Location
    if let Some(ref location) = event.location {
        ical_event.location(location);
    }

    // Start and end times
    if event.is_all_day {
        // All-day events use DATE format (no time component)
        ical_event.all_day(event.start.date_naive());
    } else {
        ical_event.starts(event.start);
        ical_event.ends(event.end);
    }

    // Status
    let status_str = match event.status {
        EventStatus::Confirmed => "CONFIRMED",
        EventStatus::Tentative => "TENTATIVE",
        EventStatus::Cancelled => "CANCELLED",
    };
    ical_event.add_property("STATUS", status_str);

    // Recurrence rule
    if let Some(ref rrule) = event.rrule {
        ical_event.add_property("RRULE", rrule);
    }

    // Sequence number for versioning
    ical_event.sequence(event.version as u32);

    // Created and last modified timestamps
    ical_event.timestamp(event.created_at);
    // RFC 5545 requires basic ISO 8601 format for timestamps (e.g. 20240101T120000Z)
    // chrono's to_rfc3339() produces extended format which causes sync issues in Thunderbird
    let last_modified_str = event.updated_at.format("%Y%m%dT%H%M%SZ").to_string();
    ical_event.add_property("LAST-MODIFIED", &last_modified_str);

    // Build calendar container
    let mut calendar = Calendar::new();
<<<<<<< HEAD
    calendar.append_property(icalendar::Property::new(
        "PRODID",
        "-//Televent//Televent//EN",
    ));
=======
    // icalendar crate adds PRODID by default, do not add another one
>>>>>>> origin/main
    calendar.push(ical_event);

    Ok(calendar.to_string())
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
    use uuid::Uuid;

    fn create_test_event() -> Event {
        let now = Utc::now();
        Event {
            id: Uuid::new_v4(),
            calendar_id: Uuid::new_v4(),
            uid: "test-event-123".to_string(),
            version: 1,
            etag: "abc123".to_string(),
            summary: "Test Event".to_string(),
            description: Some("Test Description".to_string()),
            location: Some("Test Location".to_string()),
            start: now,
            end: now + chrono::Duration::hours(1),
            is_all_day: false,
            rrule: None,
            status: EventStatus::Confirmed,
            timezone: "UTC".to_string(),
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

        let ical = event_to_ical(&event).unwrap();

        assert!(ical.contains("BEGIN:VEVENT"));
        assert!(ical.contains("UID:test-event-123"));
        // All-day events should not have time component
        assert!(ical.contains("DTSTART;"));
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
}
