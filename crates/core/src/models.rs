//! Core domain models for Televent
//!
//! These models represent the core business entities and map to database tables.

use crate::error::CalendarError;
use chrono::{DateTime, NaiveDate, Utc};

/// Calendar name constant (shared by all users)
pub const CALENDAR_NAME: &str = "televent";

/// Calendar color constant (shared by all users)
pub const CALENDAR_COLOR: &str = "#74c7ec";
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use typeshare::typeshare;
use utoipa::ToSchema;
use uuid::Uuid;

/// User ID newtype wrapping Telegram's permanent numeric ID
///
/// This serves as the primary identifier for both users and their calendars,
/// since each user has exactly one calendar.
#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String, example = "123456789")]
pub struct UserId(#[typeshare(serialized_as = "string")] pub i64);

impl UserId {
    /// Create a new UserId from a Telegram ID
    pub fn new(telegram_id: i64) -> Self {
        Self(telegram_id)
    }

    /// Get the inner i64 value
    pub fn inner(self) -> i64 {
        self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<UserId> for i64 {
    fn from(id: UserId) -> Self {
        id.0
    }
}

// SQLx support for UserId
impl<'r> sqlx::Decode<'r, sqlx::Postgres> for UserId {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let inner = <i64 as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(UserId(inner))
    }
}

impl sqlx::Type<sqlx::Postgres> for UserId {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <i64 as sqlx::Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        <i64 as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for UserId {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync + 'static>> {
        <i64 as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

/// Timezone newtype wrapping chrono_tz::Tz with SQLx and Serde support
///
/// Stored in database as TEXT (IANA timezone name like "America/New_York")
#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ToSchema)]
#[schema(value_type = String, example = "UTC")]
pub struct Timezone(#[typeshare(serialized_as = "string")] pub Tz);

impl Timezone {
    /// Create a new Timezone from an IANA timezone name
    pub fn new(tz: Tz) -> Self {
        Self(tz)
    }

    /// Parse timezone from string
    pub fn parse(s: &str) -> Result<Self, String> {
        Tz::from_str(s)
            .map(Timezone)
            .map_err(|_| format!("Invalid timezone: {}", s))
    }

    /// Get the inner Tz value
    pub fn inner(self) -> Tz {
        self.0
    }
}

impl Default for Timezone {
    fn default() -> Self {
        Timezone(Tz::UTC)
    }
}

impl fmt::Display for Timezone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.name())
    }
}

impl FromStr for Timezone {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for Timezone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.name())
    }
}

impl<'de> Deserialize<'de> for Timezone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Timezone::parse(&s).map_err(serde::de::Error::custom)
    }
}

// SQLx support for Timezone
impl<'r> sqlx::Decode<'r, sqlx::Postgres> for Timezone {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let inner = <String as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Timezone::parse(&inner).map_err(|e| e.into())
    }
}

impl sqlx::Type<sqlx::Postgres> for Timezone {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for Timezone {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync + 'static>> {
        <String as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0.name().to_string(), buf)
    }
}

/// User entity (includes calendar data since user = calendar)
///
/// The telegram_id serves as the primary key and unique identifier.
/// Calendar properties are merged into this struct since each user has exactly one calendar.
#[typeshare]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct User {
    /// Primary key: Telegram's permanent numeric ID
    #[sqlx(rename = "telegram_id")]
    #[schema(value_type = String)]
    pub id: UserId,
    /// Telegram username/handle (can change, used for CalDAV URLs)
    pub telegram_username: Option<String>,
    /// IANA timezone (e.g., "Asia/Singapore")
    pub timezone: Timezone,
    /// RFC 6578 sync token for CalDAV sync-collection
    pub sync_token: String,
    /// Collection tag for change detection
    pub ctag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Get the login identifier for CalDAV URLs
    ///
    /// Returns username if available, otherwise the numeric ID as string
    pub fn login_identifier(&self) -> String {
        self.telegram_username
            .clone()
            .unwrap_or_else(|| self.id.to_string())
    }
}

/// Event entity
#[typeshare]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Event {
    pub id: Uuid,
    /// Owner's user ID (telegram_id)
    #[schema(value_type = String)]
    pub user_id: UserId,
    pub uid: String, // iCalendar UID (stable across syncs)
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,

    // Time-based Event (Maps to Postgres TIMESTAMPTZ)
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,

    // Date-based Event (Maps to Postgres DATE)
    // Typeshare sees this as 'string' (YYYY-MM-DD)
    #[typeshare(serialized_as = "string")]
    pub start_date: Option<NaiveDate>,
    #[typeshare(serialized_as = "string")]
    pub end_date: Option<NaiveDate>,

    pub is_all_day: bool,
    pub status: EventStatus,   // CONFIRMED | TENTATIVE | CANCELLED
    pub rrule: Option<String>, // RFC 5545 recurrence rule
    pub timezone: Timezone,    // VTIMEZONE reference
    pub version: i32,          // Optimistic locking
    pub etag: String,          // HTTP ETag for conflict detection
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Event {
    pub fn start_as_timestamp(&self) -> Result<DateTime<Utc>, CalendarError> {
        if let Some(s) = self.start {
            Ok(s)
        } else {
            // Treat Date as 00:00 UTC for sorting purposes only
            let date = self.start_date.ok_or_else(|| {
                CalendarError::InvalidEventData("Event has no start time or date".to_string())
            })?;

            date.and_hms_opt(0, 0, 0)
                .map(|dt| dt.and_utc())
                .ok_or_else(|| {
                    CalendarError::InvalidEventData("Failed to create timestamp from date".to_string())
                })
        }
    }
}

/// Event status enumeration
#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "event_status", rename_all = "UPPERCASE")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

/// Device password for CalDAV authentication
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct DevicePassword {
    pub id: Uuid,
    #[schema(value_type = String)]
    pub user_id: UserId,
    #[sqlx(rename = "password_hash")]
    pub hashed_password: String, // Argon2id hash
    #[sqlx(rename = "device_name")]
    pub name: String, // User-friendly label (e.g., "iPhone")
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Outbox message for asynchronous processing
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "outbox_status", rename_all = "lowercase")]
pub enum OutboxStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Event attendee with RSVP status
#[typeshare]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct EventAttendee {
    #[typeshare(serialized_as = "string")]
    pub id: Uuid,
    #[typeshare(serialized_as = "string")]
    pub event_id: Uuid,
    pub email: String, // tg_123@televent.internal or external
    #[typeshare(serialized_as = "string")]
    pub telegram_id: Option<i64>, // Populated for internal users
    pub role: AttendeeRole,
    pub status: ParticipationStatus,
    #[typeshare(serialized_as = "string")]
    pub created_at: DateTime<Utc>,
    #[typeshare(serialized_as = "string")]
    pub updated_at: DateTime<Utc>,
}

/// Attendee role (RFC 5545 ROLE parameter)
#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "attendee_role", rename_all = "UPPERCASE")]
pub enum AttendeeRole {
    Organizer,
    Attendee,
}

/// Participation status (RFC 5545 PARTSTAT parameter)
#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "participation_status")]
pub enum ParticipationStatus {
    #[sqlx(rename = "NEEDS-ACTION")]
    #[serde(rename = "NeedsAction")]
    NeedsAction,
    #[sqlx(rename = "ACCEPTED")]
    #[serde(rename = "Accepted")]
    Accepted,
    #[sqlx(rename = "DECLINED")]
    #[serde(rename = "Declined")]
    Declined,
    #[sqlx(rename = "TENTATIVE")]
    #[serde(rename = "Tentative")]
    Tentative,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let id = UserId::new(123456789);
        assert_eq!(id.inner(), 123456789);
        assert_eq!(id.to_string(), "123456789");
    }

    #[test]
    fn test_user_id_into_i64() {
        let id = UserId::new(123456789);
        let raw: i64 = id.into();
        assert_eq!(raw, 123456789);
    }

    #[test]
    fn test_user_id_serialization() {
        let id = UserId::new(123456789);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "123456789");

        let deserialized: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_user_id_equality() {
        let id1 = UserId::new(123);
        let id2 = UserId::new(123);
        let id3 = UserId::new(456);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_user_id_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(UserId::new(123));
        set.insert(UserId::new(456));
        set.insert(UserId::new(123)); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_user_login_identifier_with_username() {
        let user = User {
            id: UserId::new(123456789),
            telegram_username: Some("testuser".to_string()),
            timezone: Timezone::default(),
            sync_token: "0".to_string(),
            ctag: "0".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(user.login_identifier(), "testuser");
    }

    #[test]
    fn test_user_login_identifier_without_username() {
        let user = User {
            id: UserId::new(123456789),
            telegram_username: None,
            timezone: Timezone::default(),
            sync_token: "0".to_string(),
            ctag: "0".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(user.login_identifier(), "123456789");
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
            user_id: UserId::new(123456789),
            uid: "test-event-uid".to_string(),
            summary: "Team Meeting".to_string(),
            description: Some("Weekly sync".to_string()),
            location: Some("Conference Room A".to_string()),
            start: Some(Utc::now()),
            end: Some(Utc::now()),
            start_date: None,
            end_date: None,
            is_all_day: false,
            status: EventStatus::Confirmed,
            rrule: None,
            timezone: Timezone::default(),
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

    #[test]
    fn test_participation_status_serialization() {
        let needs_action = ParticipationStatus::NeedsAction;
        let accepted = ParticipationStatus::Accepted;
        let declined = ParticipationStatus::Declined;
        let tentative = ParticipationStatus::Tentative;

        // Test JSON serialization
        assert_eq!(
            serde_json::to_string(&needs_action).unwrap(),
            r#""NeedsAction""#
        );
        assert_eq!(serde_json::to_string(&accepted).unwrap(), r#""Accepted""#);
        assert_eq!(serde_json::to_string(&declined).unwrap(), r#""Declined""#);
        assert_eq!(serde_json::to_string(&tentative).unwrap(), r#""Tentative""#);

        // Test deserialization
        let needs_action_de: ParticipationStatus =
            serde_json::from_str(r#""NeedsAction""#).unwrap();
        let accepted_de: ParticipationStatus = serde_json::from_str(r#""Accepted""#).unwrap();
        let declined_de: ParticipationStatus = serde_json::from_str(r#""Declined""#).unwrap();
        let tentative_de: ParticipationStatus = serde_json::from_str(r#""Tentative""#).unwrap();

        assert_eq!(needs_action, needs_action_de);
        assert_eq!(accepted, accepted_de);
        assert_eq!(declined, declined_de);
        assert_eq!(tentative, tentative_de);
    }

    #[test]
    fn test_attendee_role_serialization() {
        let organizer = AttendeeRole::Organizer;
        let attendee = AttendeeRole::Attendee;

        // Test JSON serialization
        assert_eq!(serde_json::to_string(&organizer).unwrap(), r#""Organizer""#);
        assert_eq!(serde_json::to_string(&attendee).unwrap(), r#""Attendee""#);

        // Test deserialization
        let organizer_de: AttendeeRole = serde_json::from_str(r#""Organizer""#).unwrap();
        let attendee_de: AttendeeRole = serde_json::from_str(r#""Attendee""#).unwrap();

        assert_eq!(organizer, organizer_de);
        assert_eq!(attendee, attendee_de);
    }

    #[test]
    fn test_event_attendee_structure() {
        let attendee = EventAttendee {
            id: Uuid::new_v4(),
            event_id: Uuid::new_v4(),
            email: "tg_123456789@televent.internal".to_string(),
            telegram_id: Some(123456789),
            role: AttendeeRole::Attendee,
            status: ParticipationStatus::NeedsAction,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Verify it serializes
        let json = serde_json::to_string(&attendee).unwrap();
        let deserialized: EventAttendee = serde_json::from_str(&json).unwrap();
        assert_eq!(attendee.email, deserialized.email);
        assert_eq!(attendee.telegram_id, deserialized.telegram_id);
        assert_eq!(attendee.role, deserialized.role);
        assert_eq!(attendee.status, deserialized.status);
    }
}
