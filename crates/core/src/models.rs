//! Core domain models for Televent
//!
//! These models represent the core business entities and map to database tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub telegram_id: i64,
    pub telegram_username: Option<String>,
    pub timezone: String, // IANA timezone (e.g., "Asia/Singapore")
    pub created_at: DateTime<Utc>,
}

/// Calendar entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct Calendar {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: String,      // Hex color for UI
    pub sync_token: String, // RFC 6578 sync token
    pub ctag: String,       // Collection tag for change detection
}

/// Event entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct Event {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub uid: String,                 // iCalendar UID (stable across syncs)
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub is_all_day: bool,
    pub status: EventStatus,    // CONFIRMED | TENTATIVE | CANCELLED
    pub rrule: Option<String>,  // RFC 5545 recurrence rule
    pub timezone: String,       // VTIMEZONE reference
    pub version: i32,           // Optimistic locking
    pub etag: String,           // HTTP ETag for conflict detection
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Event status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(sqlx::Type)]
#[sqlx(type_name = "event_status", rename_all = "UPPERCASE")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

/// Device password for CalDAV authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct DevicePassword {
    pub id: Uuid,
    pub user_id: Uuid,
    pub hashed_password: String, // Argon2id hash
    pub name: String,            // User-friendly label (e.g., "iPhone")
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Outbox message for asynchronous processing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(sqlx::Type)]
#[sqlx(type_name = "outbox_status", rename_all = "lowercase")]
pub enum OutboxStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Audit log entry for GDPR compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub action: String,     // "event_created" | "data_exported" | "account_deleted"
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}
