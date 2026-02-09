//! Error types for Televent core domain logic

use thiserror::Error;
use uuid::Uuid;

/// Core calendar domain errors
#[derive(Error, Debug)]
pub enum CalendarError {
    #[error("Event not found: {0}")]
    EventNotFound(Uuid),

    #[error("Version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: i32, actual: i32 },

    #[error("Invalid recurrence rule: {0}")]
    InvalidRRule(String),

    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),

    #[error("Invalid event data: {0}")]
    InvalidEventData(String),
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid environment variable: {0}")]
    InvalidEnvVar(String),
}

/// Result type alias for calendar operations
pub type CalendarResult<T> = Result<T, CalendarError>;
