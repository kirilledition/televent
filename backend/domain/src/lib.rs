//! Pure calendar domain rules for Televent.
//!
//! This crate intentionally has no database, HTTP, Telegram, OpenAPI, or
//! frontend type-generation dependencies. Adapters translate into these types
//! before invoking application use cases.

pub mod recurrence;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("event end time must be after start time")]
    InvalidTimedRange,
    #[error("all-day event end date must be after start date")]
    InvalidAllDayRange,
    #[error("invalid timezone: {0}")]
    InvalidTimezone(String),
    #[error("invalid recurrence rule: {0}")]
    InvalidRRule(String),
    #[error("unknown outbox kind: {0}")]
    UnknownOutboxKind(String),
    #[error("invalid outbox payload for {kind}: {reason}")]
    InvalidOutboxPayload { kind: String, reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub i64);

impl UserId {
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn inner(self) -> i64 {
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

pub const INTERNAL_EMAIL_DOMAIN: &str = "televent.internal";
pub const CALENDAR_NAME: &str = "televent";
pub const CALENDAR_COLOR: &str = "#74c7ec";

#[must_use]
pub fn internal_email_for_telegram_id(telegram_id: i64) -> String {
    format!("tg_{}@{}", telegram_id, INTERNAL_EMAIL_DOMAIN)
}

#[must_use]
pub fn parse_internal_email_telegram_id(email: &str) -> Option<i64> {
    if !email.ends_with(&format!("@{}", INTERNAL_EMAIL_DOMAIN)) {
        return None;
    }

    let local_part = email.split('@').next()?;
    let id = local_part.strip_prefix("tg_")?;
    id.parse().ok()
}

pub use recurrence::{expand_rrule, next_occurrences, validate_rrule};

pub const MAX_UID_LENGTH: usize = 256;
pub const MAX_SUMMARY_LENGTH: usize = 256;
pub const MAX_DESCRIPTION_LENGTH: usize = 10000;
pub const MAX_LOCATION_LENGTH: usize = 1024;
pub const MAX_RRULE_LENGTH: usize = 1024;

pub fn validate_length(field_name: &str, value: &str, max_len: usize) -> Result<(), String> {
    if value.len() > max_len {
        Err(format!("{} too long (max {})", field_name, max_len))
    } else {
        Ok(())
    }
}

pub fn validate_no_control_chars(field_name: &str, value: &str) -> Result<(), String> {
    if value.chars().any(|c| c.is_control() && c != '\t') {
        Err(format!("{} cannot contain control characters", field_name))
    } else {
        Ok(())
    }
}

pub fn validate_safe_multiline_text(field_name: &str, value: &str) -> Result<(), String> {
    if value
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        Err(format!("{} cannot contain control characters", field_name))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timezone(String);

impl Timezone {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        Tz::from_str(&value).map_err(|_| DomainError::InvalidTimezone(value.clone()))?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn utc() -> Self {
        Self("UTC".to_string())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Timezone {
    fn default() -> Self {
        Self::utc()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventTiming {
    Timed {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        timezone: Timezone,
    },
    AllDay {
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
}

impl EventTiming {
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::Timed { start, end, .. } if end <= start => Err(DomainError::InvalidTimedRange),
            Self::AllDay {
                start_date,
                end_date,
            } if end_date <= start_date => Err(DomainError::InvalidAllDayRange),
            _ => Ok(()),
        }
    }

    #[must_use]
    pub fn timezone(&self) -> &str {
        match self {
            Self::Timed { timezone, .. } => timezone.as_str(),
            Self::AllDay { .. } => "UTC",
        }
    }

    #[must_use]
    pub fn start_for_display(&self) -> DateTime<Utc> {
        match self {
            Self::Timed { start, .. } => *start,
            Self::AllDay { start_date, .. } => start_date
                .and_hms_opt(0, 0, 0)
                .expect("midnight is a valid time")
                .and_utc(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

impl EventStatus {
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Confirmed => "CONFIRMED",
            Self::Tentative => "TENTATIVE",
            Self::Cancelled => "CANCELLED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttendeeRole {
    Organizer,
    Attendee,
}

impl AttendeeRole {
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Organizer => "ORGANIZER",
            Self::Attendee => "ATTENDEE",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ORGANIZER" | "organizer" => Some(Self::Organizer),
            "ATTENDEE" | "attendee" => Some(Self::Attendee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipationStatus {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
}

impl ParticipationStatus {
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::NeedsAction => "NEEDS-ACTION",
            Self::Accepted => "ACCEPTED",
            Self::Declined => "DECLINED",
            Self::Tentative => "TENTATIVE",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "NEEDS-ACTION" | "needs-action" => Some(Self::NeedsAction),
            "ACCEPTED" | "accepted" | "accept" => Some(Self::Accepted),
            "DECLINED" | "declined" | "decline" => Some(Self::Declined),
            "TENTATIVE" | "tentative" | "maybe" => Some(Self::Tentative),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttendeeFingerprint {
    pub email: String,
    pub user_id: Option<i64>,
    pub role: AttendeeRole,
    pub status: ParticipationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventEtagInput {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub timing: EventTiming,
    pub status: EventStatus,
    pub rrule: Option<String>,
    pub version: i32,
    pub attendees: Vec<AttendeeFingerprint>,
}

#[must_use]
pub fn compute_event_etag(input: &EventEtagInput) -> String {
    let mut attendees = input.attendees.clone();
    attendees.sort_by(|left, right| left.email.cmp(&right.email));

    let mut hasher = Sha256::new();
    hasher.update(input.uid.as_bytes());
    hasher.update(b"|");
    hasher.update(input.summary.as_bytes());
    hasher.update(b"|");
    hasher.update(input.description.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(input.location.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(input.version.to_be_bytes());
    hasher.update(b"|");

    match &input.timing {
        EventTiming::Timed {
            start,
            end,
            timezone,
        } => {
            hasher.update(b"timed|");
            hash_datetime(&mut hasher, start);
            hasher.update(b"|");
            hash_datetime(&mut hasher, end);
            hasher.update(b"|");
            hasher.update(timezone.as_str().as_bytes());
        }
        EventTiming::AllDay {
            start_date,
            end_date,
        } => {
            hasher.update(b"all_day|");
            hasher.update(start_date.num_days_from_ce().to_be_bytes());
            hasher.update(b"|");
            hasher.update(end_date.num_days_from_ce().to_be_bytes());
        }
    }

    hasher.update(b"|");
    hasher.update(input.status.as_sql().as_bytes());
    hasher.update(b"|");
    hasher.update(input.rrule.as_deref().unwrap_or("").as_bytes());

    for attendee in attendees {
        hasher.update(b"|attendee|");
        hasher.update(attendee.email.as_bytes());
        hasher.update(b":");
        hasher.update(attendee.user_id.unwrap_or_default().to_be_bytes());
        hasher.update(b":");
        hasher.update(attendee.role.as_sql().as_bytes());
        hasher.update(b":");
        hasher.update(attendee.status.as_sql().as_bytes());
    }

    format!("{:x}", hasher.finalize())
}

fn hash_datetime(hasher: &mut Sha256, value: &DateTime<Utc>) {
    hasher.update(value.timestamp().to_be_bytes());
    hasher.update(value.timestamp_subsec_nanos().to_be_bytes());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxKind {
    InviteNotification,
    TelegramNotification,
    ExternalEmailDeferred,
    RsvpNotification,
}

impl OutboxKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InviteNotification => "invite_notification",
            Self::TelegramNotification => "telegram_notification",
            Self::ExternalEmailDeferred => "external_email_deferred",
            Self::RsvpNotification => "rsvp_notification",
        }
    }
}

impl FromStr for OutboxKind {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "invite_notification" => Ok(Self::InviteNotification),
            "telegram_notification" => Ok(Self::TelegramNotification),
            "external_email_deferred" => Ok(Self::ExternalEmailDeferred),
            "rsvp_notification" => Ok(Self::RsvpNotification),
            other => Err(DomainError::UnknownOutboxKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InviteNotification {
    pub event_id: Uuid,
    pub target_user_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramNotification {
    pub telegram_id: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalEmailDeferred {
    pub recipient_email: String,
    pub event_summary: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RsvpNotification {
    pub organizer_telegram_id: i64,
    pub attendee_name: String,
    pub event_summary: String,
    pub rsvp_status: ParticipationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutboxPayload {
    InviteNotification(InviteNotification),
    TelegramNotification(TelegramNotification),
    ExternalEmailDeferred(ExternalEmailDeferred),
    RsvpNotification(RsvpNotification),
}

impl OutboxPayload {
    #[must_use]
    pub const fn kind(&self) -> OutboxKind {
        match self {
            Self::InviteNotification(_) => OutboxKind::InviteNotification,
            Self::TelegramNotification(_) => OutboxKind::TelegramNotification,
            Self::ExternalEmailDeferred(_) => OutboxKind::ExternalEmailDeferred,
            Self::RsvpNotification(_) => OutboxKind::RsvpNotification,
        }
    }

    pub fn payload_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        match self {
            Self::InviteNotification(payload) => serde_json::to_value(payload),
            Self::TelegramNotification(payload) => serde_json::to_value(payload),
            Self::ExternalEmailDeferred(payload) => serde_json::to_value(payload),
            Self::RsvpNotification(payload) => serde_json::to_value(payload),
        }
    }

    pub fn from_parts(kind: &str, payload: serde_json::Value) -> Result<Self, DomainError> {
        let kind = OutboxKind::from_str(kind)?;
        macro_rules! decode {
            ($variant:ident, $ty:ty) => {
                serde_json::from_value::<$ty>(payload).map(Self::$variant)
            };
        }

        let decoded = match kind {
            OutboxKind::InviteNotification => decode!(InviteNotification, InviteNotification),
            OutboxKind::TelegramNotification => decode!(TelegramNotification, TelegramNotification),
            OutboxKind::ExternalEmailDeferred => {
                decode!(ExternalEmailDeferred, ExternalEmailDeferred)
            }
            OutboxKind::RsvpNotification => decode!(RsvpNotification, RsvpNotification),
        };

        decoded.map_err(|err| DomainError::InvalidOutboxPayload {
            kind: kind.as_str().to_string(),
            reason: err.to_string(),
        })
    }

    #[must_use]
    pub fn dedupe_key(&self) -> Option<String> {
        match self {
            Self::InviteNotification(payload) => Some(format!(
                "invite:{}:{}",
                payload.event_id, payload.target_user_id
            )),
            Self::ExternalEmailDeferred(payload) => Some(format!(
                "external-email-deferred:{}:{}",
                payload.recipient_email, payload.event_summary
            )),
            Self::RsvpNotification(payload) => Some(format!(
                "rsvp:{}:{}:{}",
                payload.organizer_telegram_id, payload.attendee_name, payload.event_summary
            )),
            Self::TelegramNotification(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_validation_rejects_inverted_ranges() {
        let start = "2026-01-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let timing = EventTiming::Timed {
            start,
            end: start,
            timezone: Timezone::utc(),
        };

        assert_eq!(timing.validate(), Err(DomainError::InvalidTimedRange));

        let timing = EventTiming::AllDay {
            start_date: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
        };

        assert_eq!(timing.validate(), Err(DomainError::InvalidAllDayRange));
    }

    #[test]
    fn etag_is_deterministic_and_attendee_order_independent() {
        let start = "2026-01-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let base = EventEtagInput {
            uid: "uid-1".to_string(),
            summary: "Test".to_string(),
            description: None,
            location: None,
            timing: EventTiming::Timed {
                start,
                end: start + chrono::Duration::hours(1),
                timezone: Timezone::utc(),
            },
            status: EventStatus::Confirmed,
            rrule: None,
            version: 2,
            attendees: vec![
                AttendeeFingerprint {
                    email: "b@example.com".to_string(),
                    user_id: None,
                    role: AttendeeRole::Attendee,
                    status: ParticipationStatus::NeedsAction,
                },
                AttendeeFingerprint {
                    email: "a@example.com".to_string(),
                    user_id: Some(10),
                    role: AttendeeRole::Attendee,
                    status: ParticipationStatus::Accepted,
                },
            ],
        };

        let mut reordered = base.clone();
        reordered.attendees.reverse();

        assert_eq!(compute_event_etag(&base), compute_event_etag(&reordered));
    }

    #[test]
    fn internal_email_round_trips_telegram_id() {
        let email = internal_email_for_telegram_id(123456789);

        assert_eq!(email, "tg_123456789@televent.internal");
        assert_eq!(parse_internal_email_telegram_id(&email), Some(123456789));
        assert_eq!(parse_internal_email_telegram_id("user@example.com"), None);
        assert_eq!(
            parse_internal_email_telegram_id("tg_bad@televent.internal"),
            None
        );
    }

    #[test]
    fn parses_attendee_role_and_participation_status() {
        assert_eq!(
            AttendeeRole::parse("ORGANIZER"),
            Some(AttendeeRole::Organizer)
        );
        assert_eq!(
            AttendeeRole::parse("attendee"),
            Some(AttendeeRole::Attendee)
        );
        assert_eq!(AttendeeRole::parse("chair"), None);

        assert_eq!(
            ParticipationStatus::parse("NEEDS-ACTION"),
            Some(ParticipationStatus::NeedsAction)
        );
        assert_eq!(
            ParticipationStatus::parse("accept"),
            Some(ParticipationStatus::Accepted)
        );
        assert_eq!(
            ParticipationStatus::parse("maybe"),
            Some(ParticipationStatus::Tentative)
        );
        assert_eq!(ParticipationStatus::parse("unknown"), None);
    }

    #[test]
    fn event_text_validation_rejects_unsafe_input() {
        assert!(validate_length("Summary", "short", MAX_SUMMARY_LENGTH).is_ok());
        assert!(
            validate_length(
                "Summary",
                &"x".repeat(MAX_SUMMARY_LENGTH + 1),
                MAX_SUMMARY_LENGTH
            )
            .is_err()
        );

        assert!(validate_no_control_chars("UID", "clean\tuid").is_ok());
        assert!(validate_no_control_chars("UID", "bad\nuid").is_err());

        assert!(validate_safe_multiline_text("Description", "line 1\nline 2").is_ok());
        assert!(validate_safe_multiline_text("Description", "bad\0description").is_err());
    }

    #[test]
    fn typed_outbox_rejects_wrong_payload_shape() {
        let err =
            OutboxPayload::from_parts("invite_notification", serde_json::json!({"bad": true}))
                .unwrap_err();

        assert!(matches!(err, DomainError::InvalidOutboxPayload { .. }));
    }
}
