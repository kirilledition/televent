//! Core domain models for Televent
//!
//! These models represent the core business entities and map to database tables.
//! All models use type-safe ID wrappers to prevent mixing different entity types.

use crate::types::{CalendarId, EventId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User entity representing a Telegram user
///
/// Each user is identified by their Telegram ID and can have one calendar.
/// The timezone field is used to display events in the user's local time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct User {
    /// Unique user identifier
    pub id: UserId,
    /// Telegram user ID (from Telegram API)
    pub telegram_id: i64,
    /// Telegram username (optional, can change)
    pub telegram_username: Option<String>,
    /// IANA timezone string (e.g., "Asia/Singapore", "America/New_York")
    pub timezone: String,
    /// When the user account was created
    pub created_at: DateTime<Utc>,
}

/// Calendar entity representing a user's calendar collection
///
/// Each user has exactly one calendar. The sync_token and ctag fields
/// are used for CalDAV synchronization (RFC 6578).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct Calendar {
    /// Unique calendar identifier
    pub id: CalendarId,
    /// Owner of this calendar
    pub user_id: UserId,
    /// Display name of the calendar
    pub name: String,
    /// Hex color code for UI display (e.g., "#3b82f6")
    pub color: String,
    /// RFC 6578 sync token (incremented on any change)
    pub sync_token: String,
    /// CalDAV collection tag (timestamp of last change)
    pub ctag: String,
}

/// Event entity representing a calendar event
///
/// Events follow the iCalendar (RFC 5545) specification and support
/// recurring events via RRULE. The version field is used for optimistic
/// locking to prevent concurrent update conflicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct Event {
    /// Unique event identifier
    pub id: EventId,
    /// Calendar this event belongs to
    pub calendar_id: CalendarId,
    /// iCalendar UID (stable across syncs, e.g., "event-abc@televent.app")
    pub uid: String,
    /// Event title/summary
    pub summary: String,
    /// Detailed description (optional)
    pub description: Option<String>,
    /// Location of the event (optional)
    pub location: Option<String>,
    /// Event start time (UTC)
    pub start: DateTime<Utc>,
    /// Event end time (UTC, must be >= start)
    pub end: DateTime<Utc>,
    /// Whether this is an all-day event
    pub is_all_day: bool,
    /// Event status (CONFIRMED, TENTATIVE, or CANCELLED)
    pub status: EventStatus,
    /// RFC 5545 recurrence rule (e.g., "FREQ=WEEKLY;BYDAY=MO")
    pub rrule: Option<String>,
    /// VTIMEZONE reference for the event
    pub timezone: String,
    /// Version number for optimistic locking (incremented on updates)
    pub version: i32,
    /// HTTP ETag for conflict detection (SHA256 hash of event data)
    pub etag: String,
    /// When the event was created
    pub created_at: DateTime<Utc>,
    /// When the event was last updated
    pub updated_at: DateTime<Utc>,
}

/// Event status enumeration following iCalendar specification
///
/// Maps to the STATUS property in iCalendar (RFC 5545).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(sqlx::Type)]
#[sqlx(type_name = "event_status", rename_all = "UPPERCASE")]
pub enum EventStatus {
    /// Event is confirmed and will occur
    Confirmed,
    /// Event is tentative and may change
    Tentative,
    /// Event has been cancelled
    Cancelled,
}

/// Device password for CalDAV authentication
///
/// Users can generate multiple device passwords for different CalDAV clients
/// (e.g., one for iPhone, one for Thunderbird). Each password is independently
/// revocable without affecting other devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct DevicePassword {
    /// Unique device password identifier
    pub id: Uuid,
    /// User this password belongs to
    pub user_id: UserId,
    /// Argon2id hash of the password (never store plaintext)
    pub hashed_password: String,
    /// User-friendly label (e.g., "iPhone", "Thunderbird")
    pub name: String,
    /// When the password was created
    pub created_at: DateTime<Utc>,
    /// When this password was last used for authentication
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Outbox message for asynchronous processing
///
/// Implements the Transactional Outbox pattern for reliable async operations.
/// The worker process polls this table and processes pending messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct OutboxMessage {
    /// Unique message identifier
    pub id: Uuid,
    /// Message type discriminator ("email" or "telegram_notification")
    pub message_type: String,
    /// JSON payload with message-specific data
    #[sqlx(json)]
    pub payload: serde_json::Value,
    /// Current processing status
    pub status: OutboxStatus,
    /// Number of retry attempts (max 5)
    pub retry_count: i32,
    /// When to process this message (for delayed delivery)
    pub scheduled_at: DateTime<Utc>,
    /// When the message was successfully processed
    pub processed_at: Option<DateTime<Utc>>,
}

/// Outbox message processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(sqlx::Type)]
#[sqlx(type_name = "outbox_status", rename_all = "lowercase")]
pub enum OutboxStatus {
    /// Message is waiting to be processed
    Pending,
    /// Message is currently being processed
    Processing,
    /// Message was successfully processed
    Completed,
    /// Message processing failed after max retries
    Failed,
}

/// Audit log entry for GDPR compliance and security monitoring
///
/// All security-relevant operations are logged here for compliance
/// and forensic analysis. Logs are retained for 2 years minimum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct AuditLog {
    /// Unique log entry identifier
    pub id: Uuid,
    /// User who performed the action
    pub user_id: UserId,
    /// Action performed (e.g., "event_created", "data_exported", "account_deleted")
    pub action: String,
    /// Type of entity affected (e.g., "event", "calendar", "user")
    pub entity_type: String,
    /// ID of the affected entity (if applicable)
    pub entity_id: Option<Uuid>,
    /// IP address of the client
    pub ip_address: Option<String>,
    /// User agent string from the client
    pub user_agent: Option<String>,
    /// When the action was performed
    pub created_at: DateTime<Utc>,
}
