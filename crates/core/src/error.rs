//! Error types for Televent core domain logic

use crate::types::{CalendarId, EventId, UserId};
use thiserror::Error;

/// Core calendar domain errors
#[derive(Error, Debug)]
pub enum CalendarError {
    #[error("Event not found: {0}")]
    EventNotFound(EventId),

    #[error("Calendar not found: {0}")]
    CalendarNotFound(CalendarId),

    #[error("User not found: {0}")]
    UserNotFound(UserId),

    #[error("Version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: i32, actual: i32 },

    #[error("Invalid recurrence rule: {0}")]
    InvalidRRule(String),

    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),

    #[error("Invalid event data: {0}")]
    InvalidEventData(String),

    #[error("Permission denied")]
    PermissionDenied,
}

/// Result type alias for calendar operations
pub type CalendarResult<T> = Result<T, CalendarError>;
