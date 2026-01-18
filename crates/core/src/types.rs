//! Type-safe wrappers for domain identifiers
//!
//! These newtypes prevent mixing different ID types at compile time.
//! For example, you cannot pass a UserId where a CalendarId is expected.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// User identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    /// Create a new user ID
    pub fn new() -> Self {
        UserId(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(id: Uuid) -> Self {
        UserId(id)
    }
}

impl From<UserId> for Uuid {
    fn from(id: UserId) -> Self {
        id.0
    }
}

/// Calendar identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct CalendarId(pub Uuid);

impl CalendarId {
    /// Create a new calendar ID
    pub fn new() -> Self {
        CalendarId(Uuid::new_v4())
    }
}

impl Default for CalendarId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CalendarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CalendarId {
    fn from(id: Uuid) -> Self {
        CalendarId(id)
    }
}

impl From<CalendarId> for Uuid {
    fn from(id: CalendarId) -> Self {
        id.0
    }
}

/// Event identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct EventId(pub Uuid);

impl EventId {
    /// Create a new event ID
    pub fn new() -> Self {
        EventId(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for EventId {
    fn from(id: Uuid) -> Self {
        EventId(id)
    }
}

impl From<EventId> for Uuid {
    fn from(id: EventId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_calendar_id_creation() {
        let id1 = CalendarId::new();
        let id2 = CalendarId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_event_id_creation() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_user_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let user_id = UserId::from(uuid);
        assert_eq!(Uuid::from(user_id), uuid);
    }

    #[test]
    fn test_calendar_id_display() {
        let id = CalendarId::new();
        let display = format!("{}", id);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_event_id_serialization() {
        let id = EventId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: EventId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
