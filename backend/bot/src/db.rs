//! Database operations for the bot
//!
//! Handles all database queries needed by bot command handlers

use chrono::{DateTime, NaiveDate, Utc};
use televent_application::{
    ApplicationError, CalendarIcalExport, CalendarService, ConfirmRsvpCommand,
    CreateDevicePasswordCommand, CreateEventCommand, DeviceService, EventView,
    InviteAttendeeCommand, UserId,
};
use televent_domain::{
    AttendeeRole, EventStatus as DomainEventStatus, EventTiming, ParticipationStatus, Timezone,
};
use uuid::Uuid;

/// Bot database handle
#[derive(Clone)]
pub struct BotDb {
    calendar: CalendarService,
    device: DeviceService,
}

/// Event data structure for bot display
#[derive(Debug, Clone)]
pub struct BotEvent {
    pub id: Uuid,
    pub summary: String,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub description: Option<String>,
}

impl BotEvent {
    /// Get a unified start time for display/sorting
    pub fn display_start(&self) -> DateTime<Utc> {
        if self.is_all_day {
            self.start_date
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|date_time| date_time.and_utc())
                .unwrap_or_else(Utc::now)
        } else {
            self.start.unwrap_or_else(Utc::now)
        }
    }

    /// Get timing as ParsedTiming enum
    pub fn timing(&self) -> crate::event_parser::ParsedTiming {
        if self.is_all_day {
            crate::event_parser::ParsedTiming::AllDay {
                date: self.start_date.unwrap_or_else(|| Utc::now().date_naive()),
            }
        } else {
            // Calculate duration in minutes
            let duration_minutes = if let (Some(s), Some(e)) = (self.start, self.end) {
                (e - s).num_minutes() as u32
            } else {
                60
            };
            crate::event_parser::ParsedTiming::Timed {
                start: self.start.unwrap_or_else(Utc::now),
                duration_minutes,
            }
        }
    }
}

/// Device password information for display
#[derive(Debug, Clone)]
pub struct DevicePasswordInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// User information for lookups
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub telegram_id: i64,
    #[allow(dead_code)]
    pub telegram_username: Option<String>,
}

/// Event information with ownership check
#[derive(Debug, Clone)]
pub struct EventInfo {
    pub id: Uuid,
    pub summary: String,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub user_id: UserId,
}

impl EventInfo {
    fn from_event(event: EventView, user_id: UserId) -> Self {
        let timing = timing_parts(&event.timing);
        Self {
            id: event.id,
            summary: event.summary,
            start: timing.start,
            end: timing.end,
            start_date: timing.start_date,
            end_date: timing.end_date,
            is_all_day: timing.is_all_day,
            location: event.location,
            user_id,
        }
    }
}

/// Pending invite information
#[derive(Debug, Clone)]
pub struct PendingInvite {
    pub event_id: Uuid,
    pub summary: String,
    pub start: Option<DateTime<Utc>>,
    pub start_date: Option<chrono::NaiveDate>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub organizer_username: Option<String>,
}

/// Attendee information for display
#[derive(Debug, Clone)]
pub struct AttendeeInfo {
    pub email: String,
    pub telegram_id: Option<i64>,
    pub role: String,
    pub status: String,
    pub telegram_username: Option<String>,
}

impl BotDb {
    /// Create a new database handle
    pub fn new(calendar: CalendarService, device: DeviceService) -> Self {
        Self { calendar, device }
    }

    /// Get events for a user within a date range
    pub async fn get_events_for_user(
        &self,
        telegram_id: i64,
        start_range: DateTime<Utc>,
        end_range: DateTime<Utc>,
    ) -> Result<Vec<BotEvent>, ApplicationError> {
        let events = self
            .calendar
            .list_event_views(
                UserId::new(telegram_id),
                Some(start_range),
                Some(end_range),
                None,
                None,
            )
            .await?;

        Ok(events
            .into_iter()
            .filter(|event| event.status != DomainEventStatus::Cancelled)
            .map(BotEvent::from_event)
            .collect())
    }

    /// Get all events for a user (for export)
    pub async fn get_all_events_for_user(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<BotEvent>, ApplicationError> {
        let events = self
            .calendar
            .list_event_views(UserId::new(telegram_id), None, None, None, None)
            .await?;

        Ok(events
            .into_iter()
            .filter(|event| event.status != DomainEventStatus::Cancelled)
            .map(BotEvent::from_event)
            .collect())
    }

    pub async fn export_calendar_ics(
        &self,
        telegram_id: i64,
    ) -> Result<CalendarIcalExport, ApplicationError> {
        self.calendar
            .export_calendar_ical(UserId::new(telegram_id))
            .await
    }

    /// Ensure user exists (user = calendar in new schema)
    pub async fn ensure_user_setup(
        &self,
        telegram_id: i64,
        username: Option<&str>,
    ) -> Result<(), ApplicationError> {
        self.calendar.ensure_user_setup(telegram_id, username).await
    }

    /// Generate a new device password for a user
    pub async fn generate_device_password(
        &self,
        telegram_id: i64,
        device_name: &str,
    ) -> Result<String, ApplicationError> {
        let device = self
            .device
            .create_device_password(CreateDevicePasswordCommand {
                user_id: UserId::new(telegram_id),
                username: None,
                name: device_name.to_string(),
            })
            .await?;

        Ok(device.password)
    }

    /// List all device passwords for a user
    pub async fn list_device_passwords(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<DevicePasswordInfo>, ApplicationError> {
        let devices = self
            .device
            .list_device_passwords(UserId::new(telegram_id))
            .await?;

        Ok(devices
            .into_iter()
            .map(|device| DevicePasswordInfo {
                id: device.id,
                name: device.name,
                created_at: device.created_at,
                last_used_at: device.last_used_at,
            })
            .collect())
    }

    /// Revoke (delete) a device password
    pub async fn revoke_device_password(
        &self,
        telegram_id: i64,
        device_id: Uuid,
    ) -> Result<bool, ApplicationError> {
        self.device
            .revoke_device_password(UserId::new(telegram_id), device_id)
            .await
    }

    /// Find user by Telegram username
    pub async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserInfo>, ApplicationError> {
        let username_param = username.trim_start_matches('@');
        Ok(self
            .calendar
            .get_user_identity_by_username(username_param)
            .await?
            .map(|user| UserInfo {
                telegram_id: user.id.inner(),
                telegram_username: user.username,
            }))
    }

    /// Get event info and verify ownership
    pub async fn get_event_info(
        &self,
        event_id: Uuid,
        telegram_id: i64,
    ) -> Result<Option<EventInfo>, ApplicationError> {
        match self
            .calendar
            .get_event_view(UserId::new(telegram_id), event_id)
            .await
        {
            Ok(event) => Ok(Some(EventInfo::from_event(event, UserId::new(telegram_id)))),
            Err(ApplicationError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Invite attendee to an event
    pub async fn invite_attendee(
        &self,
        event_id: Uuid,
        email: &str,
        user_id: Option<i64>,
        role: &str,
    ) -> Result<(), ApplicationError> {
        let organizer_user_id = self
            .get_event_organizer(event_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(event_id.to_string()))?;

        self.calendar
            .invite_attendee(InviteAttendeeCommand {
                organizer_user_id: UserId::new(organizer_user_id),
                event_id,
                email: email.to_string(),
                attendee_user_id: user_id.map(UserId::new),
                role: match role {
                    "ORGANIZER" => AttendeeRole::Organizer,
                    _ => AttendeeRole::Attendee,
                },
            })
            .await
    }

    /// Update RSVP status for an attendee (simple update)
    pub async fn update_rsvp_status(
        &self,
        event_id: Uuid,
        user_id: i64,
        status: &str,
    ) -> Result<bool, ApplicationError> {
        self.confirm_rsvp(event_id, user_id, status).await?;
        Ok(true)
    }

    /// Confirm RSVP status through the application transaction.
    pub async fn confirm_rsvp(
        &self,
        event_id: Uuid,
        user_id: i64,
        status: &str,
    ) -> Result<(), ApplicationError> {
        self.confirm_rsvp_named(event_id, user_id, status, format!("User_{}", user_id))
            .await
    }

    pub async fn confirm_rsvp_named(
        &self,
        event_id: Uuid,
        user_id: i64,
        status: &str,
        attendee_name: String,
    ) -> Result<(), ApplicationError> {
        let status = ParticipationStatus::parse(status).ok_or_else(|| {
            ApplicationError::BadRequest(format!("Invalid RSVP status: {status}"))
        })?;

        self.calendar
            .confirm_rsvp(ConfirmRsvpCommand {
                event_id,
                attendee_user_id: UserId::new(user_id),
                status,
                attendee_name,
            })
            .await
    }

    /// Get pending invites for a user
    pub async fn get_pending_invites(
        &self,
        telegram_id: i64,
    ) -> Result<Vec<PendingInvite>, ApplicationError> {
        let invites = self
            .calendar
            .list_pending_invites(UserId::new(telegram_id))
            .await?;

        Ok(invites
            .into_iter()
            .map(|invite| PendingInvite {
                event_id: invite.event_id,
                summary: invite.summary,
                start: invite.start,
                start_date: invite.start_date,
                is_all_day: invite.is_all_day,
                location: invite.location,
                organizer_username: invite.organizer_username,
            })
            .collect())
    }

    /// Get all attendees for an event
    pub async fn get_event_attendees(
        &self,
        event_id: Uuid,
    ) -> Result<Vec<AttendeeInfo>, ApplicationError> {
        let attendees = self.calendar.list_attendees_for_display(event_id).await?;

        Ok(attendees
            .into_iter()
            .map(|attendee| AttendeeInfo {
                email: attendee.email,
                telegram_id: attendee.telegram_id,
                role: attendee.role.as_sql().to_string(),
                status: attendee.status.as_sql().to_string(),
                telegram_username: attendee.telegram_username,
            })
            .collect())
    }

    /// Get event organizer's telegram_id
    pub async fn get_event_organizer(
        &self,
        event_id: Uuid,
    ) -> Result<Option<i64>, ApplicationError> {
        Ok(self
            .calendar
            .get_event_owner_id(event_id)
            .await?
            .map(UserId::inner))
    }

    /// Create a new event
    #[allow(clippy::too_many_arguments)]
    pub async fn create_event(
        &self,
        telegram_id: i64,
        uid: &str,
        summary: &str,
        description: Option<&str>,
        location: Option<&str>,
        timing: crate::event_parser::ParsedTiming,
        timezone: &str,
    ) -> Result<BotEvent, ApplicationError> {
        let domain_timing = match timing {
            crate::event_parser::ParsedTiming::Timed {
                start,
                duration_minutes,
            } => {
                let end = start + chrono::Duration::minutes(i64::from(duration_minutes));
                EventTiming::Timed {
                    start,
                    end,
                    timezone: Timezone::parse(timezone.to_string()).unwrap_or_default(),
                }
            }
            crate::event_parser::ParsedTiming::AllDay { date } => {
                let end_date = date + chrono::Duration::days(1);
                EventTiming::AllDay {
                    start_date: date,
                    end_date,
                }
            }
        };

        let event = self
            .calendar
            .create_event_view(CreateEventCommand {
                user_id: UserId::new(telegram_id),
                username: None,
                uid: uid.to_string(),
                summary: summary.to_string(),
                description: description.map(str::to_string),
                location: location.map(str::to_string),
                timing: domain_timing,
                status: DomainEventStatus::Confirmed,
                rrule: None,
            })
            .await?;

        Ok(BotEvent::from_event(event))
    }
}

impl BotEvent {
    fn from_event(event: EventView) -> Self {
        let timing = timing_parts(&event.timing);
        Self {
            id: event.id,
            summary: event.summary,
            start: timing.start,
            end: timing.end,
            start_date: timing.start_date,
            end_date: timing.end_date,
            is_all_day: timing.is_all_day,
            location: event.location,
            description: event.description,
        }
    }
}

struct TimingParts {
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    is_all_day: bool,
}

fn timing_parts(timing: &EventTiming) -> TimingParts {
    match timing {
        EventTiming::Timed { start, end, .. } => TimingParts {
            start: Some(*start),
            end: Some(*end),
            start_date: None,
            end_date: None,
            is_all_day: false,
        },
        EventTiming::AllDay {
            start_date,
            end_date,
        } => TimingParts {
            start: None,
            end: None,
            start_date: Some(*start_date),
            end_date: Some(*end_date),
            is_all_day: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use sqlx::PgPool;

    fn bot_db(pool: PgPool) -> BotDb {
        BotDb::new(
            CalendarService::new(televent_storage::calendar::CalendarRepository::new(
                pool.clone(),
            )),
            DeviceService::new(televent_storage::device::DeviceRepository::new(pool)),
        )
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_user_setup(pool: PgPool) {
        let db = bot_db(pool);
        let telegram_id = 1001;

        // Ensure user is set up
        let result = db.ensure_user_setup(telegram_id, Some("testuser")).await;
        assert!(result.is_ok());

        // Ensure idempotency
        let result2 = db.ensure_user_setup(telegram_id, Some("testuser")).await;
        assert!(result2.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_event_lifecycle(pool: PgPool) {
        let db = bot_db(pool);
        let telegram_id = 1002;
        db.ensure_user_setup(telegram_id, None)
            .await
            .expect("Failed setup");

        let start = Utc::now();
        let uid = format!("{}", Uuid::new_v4());

        // Create event
        let event = db
            .create_event(
                telegram_id,
                &uid,
                "Test Event",
                Some("Description"),
                Some("Location"),
                crate::event_parser::ParsedTiming::Timed {
                    start,
                    duration_minutes: 60,
                },
                "UTC",
            )
            .await
            .expect("Failed to create event");

        assert_eq!(event.summary, "Test Event");
        assert_eq!(event.location.as_deref(), Some("Location"));

        // Retrieve event via get_events_for_user (checking range)
        let events = db
            .get_events_for_user(
                telegram_id,
                start - Duration::minutes(10),
                start + Duration::hours(2),
            )
            .await
            .expect("Failed to get events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Test Event");

        // Retrieve event via get_all_events_for_user
        let all_events = db
            .get_all_events_for_user(telegram_id)
            .await
            .expect("Failed to get all events");
        assert_eq!(all_events.len(), 1);

        // Retrieve event info
        let info = db
            .get_event_info(event.id, telegram_id)
            .await
            .expect("Failed to get info");
        assert!(info.is_some());
        assert_eq!(info.unwrap().summary, "Test Event");

        // Check non-existent event or wrong user
        let info_none = db
            .get_event_info(event.id, 99999)
            .await
            .expect("Failed to query");
        assert!(info_none.is_none());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_device_password_management(pool: PgPool) {
        let db = bot_db(pool);
        let telegram_id = 1003;
        db.ensure_user_setup(telegram_id, None)
            .await
            .expect("Setup failed");

        // Create device password
        let password = db
            .generate_device_password(telegram_id, "Test Device")
            .await
            .expect("Generate failed");
        assert_eq!(password.len(), 24);

        // List passwords
        let devices = db
            .list_device_passwords(telegram_id)
            .await
            .expect("List failed");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Test Device");

        // Revoke password
        let revoked = db
            .revoke_device_password(telegram_id, devices[0].id)
            .await
            .expect("Revoke failed");
        assert!(revoked);

        // Revoke again (should be false)
        let revoked2 = db
            .revoke_device_password(telegram_id, devices[0].id)
            .await
            .expect("Revoke2 failed");
        assert!(!revoked2);

        // List again
        let devices_after = db
            .list_device_passwords(telegram_id)
            .await
            .expect("List failed");
        assert!(devices_after.is_empty());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_invites_and_rsvps(pool: PgPool) {
        let db = bot_db(pool);
        let organizer_id = 1004;
        let attendee_id = 1005;

        db.ensure_user_setup(organizer_id, Some("organizer"))
            .await
            .expect("Org setup failed");
        db.ensure_user_setup(attendee_id, Some("attendee"))
            .await
            .expect("Att setup failed");

        // Organizer creates event
        let start = Utc::now();
        let uid = format!("{}", Uuid::new_v4());

        let event = db
            .create_event(
                organizer_id,
                &uid,
                "Party",
                None,
                None,
                crate::event_parser::ParsedTiming::Timed {
                    start,
                    duration_minutes: 60,
                },
                "UTC",
            )
            .await
            .expect("Create event failed");

        // Invite attendee
        db.invite_attendee(
            event.id,
            "attendee@example.com",
            Some(attendee_id),
            "ATTENDEE",
        )
        .await
        .expect("Invite failed");

        // Check pending invites
        let pending = db
            .get_pending_invites(attendee_id)
            .await
            .expect("Get pending failed");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].summary, "Party");

        // RSVPs
        let updated = db
            .update_rsvp_status(event.id, attendee_id, "ACCEPTED")
            .await
            .expect("RSVP failed");
        assert!(updated);

        // Check pending again (should be empty as status changed)
        let pending_after = db
            .get_pending_invites(attendee_id)
            .await
            .expect("Get pending failed");
        assert!(pending_after.is_empty());

        // Get attendees list
        let attendees = db
            .get_event_attendees(event.id)
            .await
            .expect("Get attendees");
        assert!(!attendees.is_empty());
        let att = attendees
            .iter()
            .find(|a| a.telegram_id == Some(attendee_id))
            .expect("Attendee not found");
        assert_eq!(att.status, "ACCEPTED");

        // Get organizer id from event
        let org_id_check = db
            .get_event_organizer(event.id)
            .await
            .expect("Get org")
            .unwrap();
        assert_eq!(org_id_check, organizer_id);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_confirm_rsvp_transaction(pool: PgPool) {
        let db = bot_db(pool.clone());
        let organizer_id = 2004;
        let attendee_id = 2005;

        db.ensure_user_setup(organizer_id, Some("organizer_orig"))
            .await
            .expect("Org setup failed");
        db.ensure_user_setup(attendee_id, Some("attendee_orig"))
            .await
            .expect("Att setup failed");

        let start = Utc::now();
        let uid = format!("{}", Uuid::new_v4());

        let event = db
            .create_event(
                organizer_id,
                &uid,
                "Transaction Test",
                None,
                None,
                crate::event_parser::ParsedTiming::Timed {
                    start,
                    duration_minutes: 60,
                },
                "UTC",
            )
            .await
            .expect("Create event failed");

        // Invite attendee
        db.invite_attendee(event.id, "att@tx.com", Some(attendee_id), "ATTENDEE")
            .await
            .expect("Invite failed");

        // Get initial values
        let (initial_sync_token, initial_ctag, initial_version, initial_sync_version): (
            i64,
            i64,
            i32,
            i64,
        ) = sqlx::query_as(
            "SELECT u.sync_token, u.ctag, e.version, e.sync_version FROM users u JOIN events e ON u.telegram_id = e.user_id WHERE e.id = $1"
        )
        .bind(event.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // Perform confirm_rsvp
        db.confirm_rsvp(event.id, attendee_id, "TENTATIVE")
            .await
            .expect("confirm_rsvp failed");

        // Verify updates
        let (new_sync_token, new_ctag, new_version, new_sync_version): (i64, i64, i32, i64) =
            sqlx::query_as(
                "SELECT u.sync_token, u.ctag, e.version, e.sync_version FROM users u JOIN events e ON u.telegram_id = e.user_id WHERE e.id = $1"
            )
        .bind(event.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            initial_ctag, initial_sync_token,
            "Initial CTAG should track sync token"
        );
        assert_eq!(
            new_sync_token,
            initial_sync_token + 1,
            "Sync token should increment exactly once"
        );
        assert_eq!(new_ctag, new_sync_token, "CTAG should track sync token");
        assert_eq!(
            new_version,
            initial_version + 1,
            "Version should have incremented"
        );
        assert_eq!(
            new_sync_version,
            initial_sync_version + 1,
            "Event sync_version should match the one calendar sync bump"
        );

        // Verify attendee status
        let attendees = db.get_event_attendees(event.id).await.unwrap();
        let att = attendees
            .iter()
            .find(|a| a.telegram_id == Some(attendee_id))
            .unwrap();
        assert_eq!(att.status, "TENTATIVE");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_user_lookup(pool: PgPool) {
        let db = bot_db(pool);
        let telegram_id = 1006;
        let username = "someuser";

        db.ensure_user_setup(telegram_id, Some(username))
            .await
            .expect("Setup failed");

        // Lookup user
        let user = db
            .find_user_by_username(username)
            .await
            .expect("Lookup failed");
        assert!(user.is_some());
        assert_eq!(user.unwrap().telegram_id, telegram_id);
    }

    #[test]
    fn test_bot_db_creation() {
        // This is a compile-time test to ensure BotDb can be created
    }

    #[test]
    fn test_bot_event_structure() {
        // Verify BotEvent implements required traits
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}

        assert_clone::<BotEvent>();
        assert_debug::<BotEvent>();
    }
}
