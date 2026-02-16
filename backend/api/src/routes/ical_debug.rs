#[cfg(test)]
mod tests {
    use crate::routes::ical::{event_to_ical, event_to_ical_into};
    use televent_core::models::{Event, EventAttendee, EventStatus, ParticipationStatus, UserId, Timezone, AttendeeRole};
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn debug_attendee_output() {
        let now = Utc::now();
        let event = Event {
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
        };
        let attendees = vec![
            EventAttendee {
                event_id: event.id,
                email: "test@example.com".to_string(),
                user_id: None,
                role: AttendeeRole::Attendee,
                status: ParticipationStatus::Accepted,
                created_at: now,
                updated_at: now,
            }
        ];

        let ical = event_to_ical(&event, &attendees).unwrap();
        println!("Generated iCal:\n{}", ical);
    }
}
