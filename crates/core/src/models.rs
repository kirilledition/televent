//! Core domain models for Televent
//!
//! These models represent the core business entities and map to database tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub telegram_id: i64,
    pub telegram_username: Option<String>,
    pub timezone: String, // IANA timezone (e.g., "Asia/Singapore")
    pub created_at: DateTime<Utc>,
}

/// Calendar entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Calendar {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: String,      // Hex color for UI
    pub sync_token: String, // RFC 6578 sync token
    pub ctag: String,       // Collection tag for change detection
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Event entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub uid: String, // iCalendar UID (stable across syncs)
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub is_all_day: bool,
    pub status: EventStatus,   // CONFIRMED | TENTATIVE | CANCELLED
    pub rrule: Option<String>, // RFC 5545 recurrence rule
    pub timezone: String,      // VTIMEZONE reference
    pub version: i32,          // Optimistic locking
    pub etag: String,          // HTTP ETag for conflict detection
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Event status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "event_status", rename_all = "UPPERCASE")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

/// Device password for CalDAV authentication
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DevicePassword {
    pub id: Uuid,
    pub user_id: Uuid,
    pub hashed_password: String, // Argon2id hash
    pub name: String,            // User-friendly label (e.g., "iPhone")
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Outbox message for asynchronous processing
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OutboxMessage {
    pub id: Uuid,
    pub message_type: String, // "email" | "telegram_notification"
    #[sqlx(json)]
    pub payload: serde_json::Value,
    pub status: OutboxStatus,
    pub retry_count: i32,
    pub scheduled_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

/// Outbox message status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "outbox_status", rename_all = "lowercase")]
pub enum OutboxStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Audit log entry for GDPR compliance
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub action: String, // "event_created" | "data_exported" | "account_deleted"
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: Uuid::new_v4(),
            telegram_id: 123456789,
            telegram_username: Some("testuser".to_string()),
            timezone: "America/New_York".to_string(),
            created_at: Utc::now(),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user.telegram_id, deserialized.telegram_id);
        assert_eq!(user.telegram_username, deserialized.telegram_username);
        assert_eq!(user.timezone, deserialized.timezone);
    }

    #[test]
    fn test_event_status_serialization() {
        let confirmed = EventStatus::Confirmed;
        let tentative = EventStatus::Tentative;
        let cancelled = EventStatus::Cancelled;

        // Test JSON serialization
        let confirmed_json = serde_json::to_string(&confirmed).unwrap();
        let tentative_json = serde_json::to_string(&tentative).unwrap();
        let cancelled_json = serde_json::to_string(&cancelled).unwrap();

        assert_eq!(confirmed_json, r#""Confirmed""#);
        assert_eq!(tentative_json, r#""Tentative""#);
        assert_eq!(cancelled_json, r#""Cancelled""#);

        // Test deserialization
        let confirmed_de: EventStatus = serde_json::from_str(&confirmed_json).unwrap();
        let tentative_de: EventStatus = serde_json::from_str(&tentative_json).unwrap();
        let cancelled_de: EventStatus = serde_json::from_str(&cancelled_json).unwrap();

        assert_eq!(confirmed, confirmed_de);
        assert_eq!(tentative, tentative_de);
        assert_eq!(cancelled, cancelled_de);
    }

    #[test]
    fn test_outbox_status_serialization() {
        let pending = OutboxStatus::Pending;
        let processing = OutboxStatus::Processing;
        let completed = OutboxStatus::Completed;
        let failed = OutboxStatus::Failed;

        // Test JSON serialization
        assert_eq!(serde_json::to_string(&pending).unwrap(), r#""Pending""#);
        assert_eq!(
            serde_json::to_string(&processing).unwrap(),
            r#""Processing""#
        );
        assert_eq!(serde_json::to_string(&completed).unwrap(), r#""Completed""#);
        assert_eq!(serde_json::to_string(&failed).unwrap(), r#""Failed""#);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event {
            id: Uuid::new_v4(),
            calendar_id: Uuid::new_v4(),
            uid: "test-event-uid".to_string(),
            summary: "Team Meeting".to_string(),
            description: Some("Weekly sync".to_string()),
            location: Some("Conference Room A".to_string()),
            start: Utc::now(),
            end: Utc::now(),
            is_all_day: false,
            status: EventStatus::Confirmed,
            rrule: None,
            timezone: "UTC".to_string(),
            version: 1,
            etag: "abc123".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();

        assert_eq!(event.uid, deserialized.uid);
        assert_eq!(event.summary, deserialized.summary);
        assert_eq!(event.status, deserialized.status);
        assert_eq!(event.version, deserialized.version);
    }

    #[test]
    fn test_outbox_message_payload() {
        use serde_json::json;

        let payload = json!({
            "telegram_id": 123456789,
            "message": "Test notification"
        });

        let outbox = OutboxMessage {
            id: Uuid::new_v4(),
            message_type: "telegram_notification".to_string(),
            payload: payload.clone(),
            status: OutboxStatus::Pending,
            retry_count: 0,
            scheduled_at: Utc::now(),
            processed_at: None,
        };

        // Test that payload is preserved
        assert_eq!(outbox.payload["telegram_id"], 123456789);
        assert_eq!(outbox.payload["message"], "Test notification");
    }
}
