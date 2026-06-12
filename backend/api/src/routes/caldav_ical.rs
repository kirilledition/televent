//! CalDAV iCalendar request parsing.
//!
//! This module is protocol-adapter code: it translates an incoming CalDAV
//! VEVENT into application command data, while the route handler stays focused
//! on HTTP/auth/service orchestration.

use std::collections::HashMap;

use ical::parser::ical::component::IcalEvent;
use televent_application::{AttendeeCommand, PutEventCommand, UserId, ical as app_ical};
use televent_domain::{
    AttendeeRole, EventStatus, EventTiming, MAX_DESCRIPTION_LENGTH, MAX_LOCATION_LENGTH,
    MAX_RRULE_LENGTH, MAX_SUMMARY_LENGTH, MAX_UID_LENGTH, ParticipationStatus, Timezone,
    parse_internal_email_telegram_id, validate_length, validate_no_control_chars, validate_rrule,
    validate_safe_multiline_text,
};

use crate::error::ApiError;

#[derive(Debug, Clone)]
pub struct ParsedCalDavEvent {
    uid: String,
    summary: String,
    description: Option<String>,
    location: Option<String>,
    timing: EventTiming,
    status: EventStatus,
    rrule: Option<String>,
    attendees: Vec<AttendeeCommand>,
}

impl ParsedCalDavEvent {
    pub fn into_put_command(
        self,
        user_id: UserId,
        expected_etag: Option<String>,
    ) -> PutEventCommand {
        PutEventCommand {
            user_id,
            uid: self.uid,
            summary: self.summary,
            description: self.description,
            location: self.location,
            timing: self.timing,
            status: self.status,
            rrule: self.rrule,
            expected_etag,
            attendees: self.attendees,
        }
    }
}

pub fn parse_put_event(
    ical_str: &str,
    expected_uid: &str,
    organizer_user_id: UserId,
) -> Result<ParsedCalDavEvent, ApiError> {
    let parsed_calendar = ical::IcalParser::new(std::io::Cursor::new(ical_str))
        .next()
        .ok_or_else(|| ApiError::BadRequest("Empty calendar".to_string()))?
        .map_err(|err| ApiError::BadRequest(format!("Failed to parse calendar: {err}")))?;

    let event = parsed_calendar
        .events
        .first()
        .ok_or_else(|| ApiError::BadRequest("No event found in calendar".to_string()))?;

    let (uid, summary, description, location, start, end, is_all_day, rrule, status, timezone) =
        app_ical::ical_to_event_data(event)?;

    validate_event_fields(&uid, &summary, &description, &location, &rrule)?;

    if uid != expected_uid {
        return Err(ApiError::BadRequest(format!(
            "UID mismatch: {uid} != {expected_uid}"
        )));
    }

    let timing = if is_all_day {
        EventTiming::AllDay {
            start_date: start.date_naive(),
            end_date: end.date_naive(),
        }
    } else {
        EventTiming::Timed {
            start,
            end,
            timezone: Timezone::parse(timezone).unwrap_or_default(),
        }
    };

    Ok(ParsedCalDavEvent {
        uid,
        summary,
        description,
        location,
        timing,
        status,
        rrule,
        attendees: extract_attendees(event, organizer_user_id),
    })
}

fn validate_event_fields(
    uid: &str,
    summary: &str,
    description: &Option<String>,
    location: &Option<String>,
    rrule: &Option<String>,
) -> Result<(), ApiError> {
    validate_length("UID", uid, MAX_UID_LENGTH).map_err(ApiError::BadRequest)?;
    validate_no_control_chars("UID", uid).map_err(ApiError::BadRequest)?;

    validate_length("Summary", summary, MAX_SUMMARY_LENGTH).map_err(ApiError::BadRequest)?;
    validate_no_control_chars("Summary", summary).map_err(ApiError::BadRequest)?;

    if let Some(description) = description {
        validate_length("Description", description, MAX_DESCRIPTION_LENGTH)
            .map_err(ApiError::BadRequest)?;
        validate_safe_multiline_text("Description", description).map_err(ApiError::BadRequest)?;
    }

    if let Some(location) = location {
        validate_length("Location", location, MAX_LOCATION_LENGTH).map_err(ApiError::BadRequest)?;
        validate_no_control_chars("Location", location).map_err(ApiError::BadRequest)?;
    }

    if let Some(rrule) = rrule {
        validate_length("RRule", rrule, MAX_RRULE_LENGTH).map_err(ApiError::BadRequest)?;
        validate_no_control_chars("RRule", rrule).map_err(ApiError::BadRequest)?;
        validate_rrule(rrule).map_err(|err| ApiError::BadRequest(err.to_string()))?;
    }

    Ok(())
}

fn extract_attendees(event: &IcalEvent, organizer_user_id: UserId) -> Vec<AttendeeCommand> {
    let mut attendees = HashMap::new();

    for property in &event.properties {
        if property.name == "ATTENDEE"
            && let Some(value) = &property.value
        {
            let email = value.trim_start_matches("mailto:");
            let user_id = parse_internal_email_telegram_id(email).map(UserId::new);
            if user_id == Some(organizer_user_id) {
                continue;
            }

            attendees.insert(
                email.to_string(),
                AttendeeCommand {
                    email: email.to_string(),
                    user_id,
                    role: AttendeeRole::Attendee,
                    status: attendee_participation_status(property),
                },
            );
        }
    }

    attendees.into_values().collect()
}

fn attendee_participation_status(property: &ical::property::Property) -> ParticipationStatus {
    property
        .params
        .as_ref()
        .and_then(|params| {
            params
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("PARTSTAT"))
        })
        .and_then(|(_, values)| values.first())
        .and_then(|value| ParticipationStatus::parse(value))
        .unwrap_or(ParticipationStatus::NeedsAction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use televent_domain::EventTiming;

    #[test]
    fn parses_timed_put_event_with_attendee_partstat() {
        let parsed = parse_put_event(
            "BEGIN:VCALENDAR\r\n\
             VERSION:2.0\r\n\
             BEGIN:VEVENT\r\n\
             UID:event-1\r\n\
             DTSTART:20240101T100000Z\r\n\
             DTEND:20240101T110000Z\r\n\
             SUMMARY:Team Sync\r\n\
             ATTENDEE;PARTSTAT=ACCEPTED:mailto:tg_2002@televent.internal\r\n\
             END:VEVENT\r\n\
             END:VCALENDAR\r\n",
            "event-1",
            UserId::new(1001),
        )
        .expect("parse put event");

        assert_eq!(parsed.uid, "event-1");
        assert_eq!(parsed.summary, "Team Sync");
        assert!(matches!(parsed.timing, EventTiming::Timed { .. }));
        assert_eq!(parsed.attendees.len(), 1);
        assert_eq!(parsed.attendees[0].user_id, Some(UserId::new(2002)));
        assert_eq!(parsed.attendees[0].status, ParticipationStatus::Accepted);
    }

    #[test]
    fn rejects_uid_mismatch() {
        let err = parse_put_event(
            "BEGIN:VCALENDAR\r\n\
             VERSION:2.0\r\n\
             BEGIN:VEVENT\r\n\
             UID:event-1\r\n\
             DTSTART:20240101T100000Z\r\n\
             SUMMARY:Team Sync\r\n\
             END:VEVENT\r\n\
             END:VCALENDAR\r\n",
            "event-2",
            UserId::new(1001),
        )
        .expect_err("expected mismatch");

        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn rejects_invalid_rrule() {
        let err = parse_put_event(
            "BEGIN:VCALENDAR\r\n\
             VERSION:2.0\r\n\
             BEGIN:VEVENT\r\n\
             UID:event-1\r\n\
             DTSTART:20240101T100000Z\r\n\
             SUMMARY:Team Sync\r\n\
             RRULE:INVALID=TRUE\r\n\
             END:VEVENT\r\n\
             END:VCALENDAR\r\n",
            "event-1",
            UserId::new(1001),
        )
        .expect_err("expected invalid rrule");

        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn skips_organizer_attendee() {
        let parsed = parse_put_event(
            "BEGIN:VCALENDAR\r\n\
             VERSION:2.0\r\n\
             BEGIN:VEVENT\r\n\
             UID:event-1\r\n\
             DTSTART:20240101T100000Z\r\n\
             SUMMARY:Team Sync\r\n\
             ATTENDEE:mailto:tg_1001@televent.internal\r\n\
             END:VEVENT\r\n\
             END:VCALENDAR\r\n",
            "event-1",
            UserId::new(1001),
        )
        .expect("parse put event");

        assert!(parsed.attendees.is_empty());
    }
}
