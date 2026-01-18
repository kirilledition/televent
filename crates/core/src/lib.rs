//! Televent Core - Domain logic and models
//!
//! This crate contains pure domain logic with no I/O operations.
//! All database models, business logic, and error types are defined here.

pub mod error;
pub mod models;
pub mod security;
pub mod types;

pub use error::CalendarError;
pub use types::{CalendarId, EventId, UserId};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Calendar, Event, EventStatus, User};
    use chrono::Utc;

    #[test]
    fn test_user_id_type_safety() {
        let user_id = UserId::new();
        let calendar_id = CalendarId::new();
        
        // This demonstrates type safety - these are different types
        assert_ne!(format!("{}", user_id), format!("{}", calendar_id));
    }

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: UserId::new(),
            telegram_id: 123456789,
            telegram_username: Some("testuser".to_string()),
            timezone: "America/New_York".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&user).expect("Failed to serialize user");
        let deserialized: User = serde_json::from_str(&json).expect("Failed to deserialize user");
        
        assert_eq!(user.telegram_id, deserialized.telegram_id);
        assert_eq!(user.timezone, deserialized.timezone);
    }

    #[test]
    fn test_event_status_serialization() {
        let status = EventStatus::Confirmed;
        let json = serde_json::to_string(&status).expect("Failed to serialize status");
        assert_eq!(json, r#""Confirmed""#);
        
        let deserialized: EventStatus = serde_json::from_str(&json).expect("Failed to deserialize status");
        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_calendar_creation() {
        let calendar = Calendar {
            id: CalendarId::new(),
            user_id: UserId::new(),
            name: "Personal".to_string(),
            color: "#3b82f6".to_string(),
            sync_token: "0".to_string(),
            ctag: "0".to_string(),
        };

        assert_eq!(calendar.name, "Personal");
        assert!(calendar.color.starts_with('#'));
    }

    #[test]
    fn test_event_with_recurrence() {
        let now = Utc::now();
        let event = Event {
            id: EventId::new(),
            calendar_id: CalendarId::new(),
            uid: format!("event-{}@televent.app", uuid::Uuid::new_v4()),
            summary: "Weekly Meeting".to_string(),
            description: Some("Team sync meeting".to_string()),
            location: Some("Conference Room A".to_string()),
            start: now,
            end: now + chrono::Duration::hours(1),
            is_all_day: false,
            status: EventStatus::Confirmed,
            rrule: Some("FREQ=WEEKLY;BYDAY=MO".to_string()),
            timezone: "UTC".to_string(),
            version: 1,
            etag: "abc123".to_string(),
            created_at: now,
            updated_at: now,
        };

        assert!(event.rrule.is_some());
        assert_eq!(event.status, EventStatus::Confirmed);
        assert!(event.end > event.start);
    }

    #[test]
    fn test_calendar_error_display() {
        let event_id = EventId::new();
        let error = CalendarError::EventNotFound(event_id);
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Event not found"));
        assert!(error_msg.contains(&event_id.to_string()));
    }

    #[test]
    fn test_version_conflict_error() {
        let error = CalendarError::VersionConflict {
            expected: 5,
            actual: 3,
        };
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Version conflict"));
        assert!(error_msg.contains("expected 5"));
        assert!(error_msg.contains("got 3"));
    }
}
