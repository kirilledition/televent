//! Application use cases and transaction boundaries for Televent.

mod device;
mod health;
pub mod ical;

pub use device::{
    CreateDevicePasswordCommand, CreatedDevicePassword, DevicePasswordView, DeviceService,
    PASSWORD_LEN, validate_device_name,
};
pub use health::HealthService;
pub use televent_domain::UserId;

use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;
use televent_domain::{
    AttendeeFingerprint, AttendeeRole, EventEtagInput, EventStatus, EventTiming,
    ExternalEmailDeferred, InviteNotification, OutboxPayload, ParticipationStatus,
    RsvpNotification, Timezone, compute_event_etag,
};
use televent_storage::StorageError;
use televent_storage::calendar::{
    AttendeeDisplayRecord, AttendeeWrite, CalendarRepository, Event, EventAttendee, EventTombstone,
    PendingInviteRecord, StoredEventUpdate, StoredEventWrite, User,
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<televent_domain::DomainError> for ApplicationError {
    fn from(value: televent_domain::DomainError) -> Self {
        Self::BadRequest(value.to_string())
    }
}

pub(crate) fn storage_error(err: StorageError) -> ApplicationError {
    ApplicationError::Internal(format!("Storage operation failed: {err}"))
}

#[derive(Clone)]
pub struct CalendarService {
    calendar: CalendarRepository,
}

impl CalendarService {
    #[must_use]
    pub fn new(calendar: CalendarRepository) -> Self {
        Self { calendar }
    }

    pub async fn ensure_user_setup(
        &self,
        telegram_id: i64,
        username: Option<&str>,
    ) -> Result<(), ApplicationError> {
        self.get_or_create_user(telegram_id, username).await?;
        Ok(())
    }

    pub async fn get_or_create_user(
        &self,
        telegram_id: i64,
        username: Option<&str>,
    ) -> Result<UserIdentity, ApplicationError> {
        let user = self
            .calendar
            .ensure_user(telegram_id, username)
            .await
            .map_err(storage_error)?;
        Ok(UserIdentity::from(user))
    }

    async fn get_user_by_id(&self, user_id: UserId) -> Result<Option<User>, ApplicationError> {
        self.calendar
            .get_user_by_id(user_id)
            .await
            .map_err(storage_error)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, ApplicationError> {
        self.calendar
            .get_user_by_username(username)
            .await
            .map_err(storage_error)
    }

    pub async fn get_user_identity_by_id(
        &self,
        user_id: UserId,
    ) -> Result<Option<UserIdentity>, ApplicationError> {
        Ok(self.get_user_by_id(user_id).await?.map(UserIdentity::from))
    }

    pub async fn get_user_identity_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserIdentity>, ApplicationError> {
        Ok(self
            .get_user_by_username(username)
            .await?
            .map(UserIdentity::from))
    }

    pub async fn resolve_caldav_user(
        &self,
        identifier: &str,
    ) -> Result<Option<CalDavUser>, ApplicationError> {
        let user = if let Ok(telegram_id) = identifier.parse::<i64>() {
            self.get_user_by_id(UserId::new(telegram_id)).await?
        } else {
            self.get_user_by_username(identifier).await?
        };

        Ok(user.map(CalDavUser::from))
    }

    async fn get_event(&self, user_id: UserId, event_id: Uuid) -> Result<Event, ApplicationError> {
        self.calendar
            .get_event_by_id(user_id, event_id)
            .await
            .map_err(storage_error)?
            .ok_or_else(|| ApplicationError::NotFound(format!("Event not found: {event_id}")))
    }

    pub async fn get_event_view(
        &self,
        user_id: UserId,
        event_id: Uuid,
    ) -> Result<EventView, ApplicationError> {
        EventView::try_from(self.get_event(user_id, event_id).await?)
    }

    pub async fn render_event_ical(
        &self,
        user_id: UserId,
        event_id: Uuid,
    ) -> Result<RenderedEventIcal, ApplicationError> {
        let event = self.get_event(user_id, event_id).await?;
        self.render_ical_for_event(event).await
    }

    async fn get_event_by_id_any(&self, event_id: Uuid) -> Result<Option<Event>, ApplicationError> {
        self.calendar
            .get_event_by_id_any(event_id)
            .await
            .map_err(storage_error)
    }

    pub async fn get_event_owner_id(
        &self,
        event_id: Uuid,
    ) -> Result<Option<UserId>, ApplicationError> {
        Ok(self
            .get_event_by_id_any(event_id)
            .await?
            .map(|event| event.user_id))
    }

    pub async fn get_event_view_by_id_any(
        &self,
        event_id: Uuid,
    ) -> Result<Option<EventView>, ApplicationError> {
        self.get_event_by_id_any(event_id)
            .await?
            .map(EventView::try_from)
            .transpose()
    }

    async fn get_events_by_ids_any(
        &self,
        event_ids: &[Uuid],
    ) -> Result<Vec<Event>, ApplicationError> {
        self.calendar
            .get_events_by_ids_any(event_ids)
            .await
            .map_err(storage_error)
    }

    pub async fn get_event_views_by_ids_any(
        &self,
        event_ids: &[Uuid],
    ) -> Result<Vec<EventView>, ApplicationError> {
        self.get_events_by_ids_any(event_ids)
            .await?
            .into_iter()
            .map(EventView::try_from)
            .collect()
    }

    pub async fn list_caldav_event_metadata(
        &self,
        user_id: UserId,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<Vec<CalDavEventMetadata>, ApplicationError> {
        Ok(self
            .list_events(user_id, start, end, None, None)
            .await?
            .iter()
            .map(CalDavEventMetadata::from)
            .collect())
    }

    async fn get_event_by_uid(
        &self,
        user_id: UserId,
        uid: &str,
    ) -> Result<Option<Event>, ApplicationError> {
        self.calendar
            .get_event_by_uid(user_id, uid)
            .await
            .map_err(storage_error)
    }

    pub async fn render_event_ical_by_uid(
        &self,
        user_id: UserId,
        uid: &str,
    ) -> Result<Option<RenderedEventIcal>, ApplicationError> {
        let Some(event) = self.get_event_by_uid(user_id, uid).await? else {
            return Ok(None);
        };

        Ok(Some(self.render_ical_for_event(event).await?))
    }

    async fn get_events_by_uids(
        &self,
        user_id: UserId,
        uids: &[&str],
    ) -> Result<Vec<Event>, ApplicationError> {
        self.calendar
            .get_events_by_uids(user_id, uids)
            .await
            .map_err(storage_error)
    }

    async fn get_event_attendees(
        &self,
        event_id: Uuid,
    ) -> Result<Vec<EventAttendee>, ApplicationError> {
        self.calendar
            .get_event_attendees(event_id)
            .await
            .map_err(storage_error)
    }

    async fn render_ical_for_event(
        &self,
        event: Event,
    ) -> Result<RenderedEventIcal, ApplicationError> {
        let attendees = self.get_event_attendees(event.id).await?;
        let etag = event.etag.clone();
        let body = crate::ical::event_to_ical(
            &ical_event_render_from_event(&event)?,
            &ical_attendees(&attendees),
        )?;

        Ok(RenderedEventIcal { etag, body })
    }

    async fn get_event_attendees_bulk(
        &self,
        event_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<EventAttendee>>, ApplicationError> {
        self.calendar
            .get_event_attendees_bulk(event_ids)
            .await
            .map_err(storage_error)
    }

    pub async fn list_pending_invites(
        &self,
        user_id: UserId,
    ) -> Result<Vec<PendingInviteView>, ApplicationError> {
        Ok(self
            .calendar
            .list_pending_invites(user_id)
            .await
            .map_err(storage_error)?
            .into_iter()
            .map(PendingInviteView::from)
            .collect())
    }

    pub async fn list_attendees_for_display(
        &self,
        event_id: Uuid,
    ) -> Result<Vec<AttendeeDisplayView>, ApplicationError> {
        self.calendar
            .list_attendees_for_display(event_id)
            .await
            .map_err(storage_error)?
            .into_iter()
            .map(AttendeeDisplayView::try_from)
            .collect()
    }

    async fn list_events(
        &self,
        user_id: UserId,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Event>, ApplicationError> {
        self.calendar
            .list_events(user_id, start, end, limit, offset)
            .await
            .map_err(storage_error)
    }

    pub async fn list_event_views(
        &self,
        user_id: UserId,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<EventView>, ApplicationError> {
        self.list_events(user_id, start, end, limit, offset)
            .await?
            .into_iter()
            .map(EventView::try_from)
            .collect()
    }

    pub async fn export_calendar_ical(
        &self,
        user_id: UserId,
    ) -> Result<CalendarIcalExport, ApplicationError> {
        let events = self.list_events(user_id, None, None, None, None).await?;
        let active_events = events
            .into_iter()
            .filter(|event| event.status != EventStatus::Cancelled)
            .collect::<Vec<_>>();
        let event_ids = active_events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>();
        let attendees_by_event = self.get_event_attendees_bulk(&event_ids).await?;

        let mut render_events = Vec::with_capacity(active_events.len());
        for event in active_events {
            let attendees = attendees_by_event
                .get(&event.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            render_events.push(crate::ical::IcalCalendarEventRender {
                event: ical_event_render_from_event(&event)?,
                attendees: ical_attendees(attendees),
            });
        }

        let body = crate::ical::calendar_to_ical(
            &render_events,
            Some("Televent Calendar"),
            Some("Exported from Televent Telegram Bot"),
        )?;

        Ok(CalendarIcalExport {
            event_count: render_events.len(),
            body,
        })
    }

    async fn list_events_since_sync(
        &self,
        user_id: UserId,
        sync_token: i64,
    ) -> Result<Vec<Event>, ApplicationError> {
        self.calendar
            .list_events_since_sync(user_id, sync_token)
            .await
            .map_err(storage_error)
    }

    async fn list_tombstones_since_sync(
        &self,
        user_id: UserId,
        sync_token: i64,
    ) -> Result<Vec<EventTombstone>, ApplicationError> {
        self.calendar
            .list_tombstones_since(user_id, sync_token)
            .await
            .map_err(storage_error)
    }

    async fn list_events_with_attendees(
        &self,
        user_id: UserId,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<CalendarEventsWithAttendees, ApplicationError> {
        let events = self
            .calendar
            .list_events(user_id, start, end, None, None)
            .await
            .map_err(storage_error)?;
        self.attach_attendees(events).await
    }

    pub async fn list_caldav_event_resources(
        &self,
        user_id: UserId,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<Vec<CalDavEventResource>, ApplicationError> {
        let event_collection = self.list_events_with_attendees(user_id, start, end).await?;
        render_caldav_event_resources(
            event_collection.events,
            &event_collection.attendees_by_event,
        )
    }

    async fn list_events_by_uids_with_attendees(
        &self,
        user_id: UserId,
        uids: &[&str],
    ) -> Result<CalendarEventsWithAttendees, ApplicationError> {
        let events = self.get_events_by_uids(user_id, uids).await?;
        self.attach_attendees(events).await
    }

    pub async fn list_caldav_event_resources_by_uids(
        &self,
        user_id: UserId,
        uids: &[&str],
    ) -> Result<Vec<CalDavEventResource>, ApplicationError> {
        let event_collection = self
            .list_events_by_uids_with_attendees(user_id, uids)
            .await?;
        Ok(render_caldav_event_resources_lossy(
            event_collection.events,
            &event_collection.attendees_by_event,
        ))
    }

    async fn list_sync_changes(
        &self,
        user_id: UserId,
        sync_token: Option<&str>,
    ) -> Result<CalendarSyncChanges, ApplicationError> {
        let last_sync_token = parse_calendar_sync_token(sync_token);
        let events = if last_sync_token == 0 {
            self.list_events(user_id, None, None, None, None).await?
        } else {
            self.list_events_since_sync(user_id, last_sync_token)
                .await?
        };
        let CalendarEventsWithAttendees {
            events,
            attendees_by_event,
        } = self.attach_attendees(events).await?;
        let tombstones = if last_sync_token == 0 {
            Vec::new()
        } else {
            self.list_tombstones_since_sync(user_id, last_sync_token)
                .await?
        };

        Ok(CalendarSyncChanges {
            last_sync_token,
            events,
            tombstones,
            attendees_by_event,
        })
    }

    pub async fn list_caldav_sync_changes(
        &self,
        user_id: UserId,
        sync_token: Option<&str>,
    ) -> Result<CalDavSyncChanges, ApplicationError> {
        let sync_changes = self.list_sync_changes(user_id, sync_token).await?;
        let events =
            render_caldav_event_resources(sync_changes.events, &sync_changes.attendees_by_event)?;
        let tombstones = sync_changes
            .tombstones
            .into_iter()
            .map(CalDavTombstone::from)
            .collect();

        Ok(CalDavSyncChanges {
            last_sync_token: sync_changes.last_sync_token,
            events,
            tombstones,
        })
    }

    async fn attach_attendees(
        &self,
        events: Vec<Event>,
    ) -> Result<CalendarEventsWithAttendees, ApplicationError> {
        let event_ids: Vec<_> = events.iter().map(|event| event.id).collect();
        let attendees_by_event = self.get_event_attendees_bulk(&event_ids).await?;
        Ok(CalendarEventsWithAttendees {
            events,
            attendees_by_event,
        })
    }

    async fn create_event(&self, command: CreateEventCommand) -> Result<Event, ApplicationError> {
        command.timing.validate()?;

        let mut tx = self.calendar.begin().await.map_err(storage_error)?;
        tx.ensure_user(command.user_id.inner(), command.username.as_deref())
            .await
            .map_err(storage_error)?;

        let user_id = command.user_id;
        let sync_version = tx
            .bump_calendar_state(user_id)
            .await
            .map_err(storage_error)?;
        let version = 1;
        let etag = compute_event_etag(&EventEtagInput {
            uid: command.uid.clone(),
            summary: command.summary.clone(),
            description: command.description.clone(),
            location: command.location.clone(),
            timing: command.timing.clone(),
            status: command.status,
            rrule: command.rrule.clone(),
            version,
            attendees: Vec::new(),
        });

        let event = tx
            .insert_event(StoredEventWrite {
                user_id,
                uid: command.uid,
                summary: command.summary,
                description: command.description,
                location: command.location,
                timing: command.timing,
                status: command.status,
                rrule: command.rrule,
                version,
                sync_version,
                etag,
            })
            .await
            .map_err(storage_error)?;

        tx.commit().await.map_err(storage_error)?;
        Ok(event)
    }

    pub async fn create_event_view(
        &self,
        command: CreateEventCommand,
    ) -> Result<EventView, ApplicationError> {
        EventView::try_from(self.create_event(command).await?)
    }

    async fn update_event(&self, command: UpdateEventCommand) -> Result<Event, ApplicationError> {
        let mut tx = self.calendar.begin().await.map_err(storage_error)?;
        let user_id = command.user_id;
        let current = tx
            .get_event_by_id(user_id, command.event_id)
            .await
            .map_err(storage_error)?
            .ok_or_else(|| ApplicationError::NotFound(command.event_id.to_string()))?;

        let timing = match command.timing {
            Some(timing) => timing,
            None => timing_from_event(&current)?,
        };
        timing.validate()?;

        let status = command.status.unwrap_or(current.status);
        let summary = command.summary.unwrap_or_else(|| current.summary.clone());
        let description = command
            .description
            .unwrap_or_else(|| current.description.clone());
        let location = command.location.unwrap_or_else(|| current.location.clone());
        let rrule = command.rrule.unwrap_or_else(|| current.rrule.clone());
        let version = current.version + 1;
        let attendees = tx.list_attendees(current.id).await.map_err(storage_error)?;
        let sync_version = tx
            .bump_calendar_state(user_id)
            .await
            .map_err(storage_error)?;
        let etag = etag_for_parts(
            &current.uid,
            &summary,
            description.clone(),
            location.clone(),
            timing.clone(),
            status,
            rrule.clone(),
            version,
            &attendees,
        );

        let event = tx
            .update_event(StoredEventUpdate {
                id: current.id,
                user_id,
                summary,
                description,
                location,
                timing,
                status,
                rrule,
                version,
                sync_version,
                etag,
            })
            .await
            .map_err(storage_error)?;

        tx.commit().await.map_err(storage_error)?;
        Ok(event)
    }

    pub async fn update_event_view(
        &self,
        command: UpdateEventCommand,
    ) -> Result<EventView, ApplicationError> {
        EventView::try_from(self.update_event(command).await?)
    }

    pub async fn put_event_by_uid(
        &self,
        command: PutEventCommand,
    ) -> Result<PutEventResult, ApplicationError> {
        command.timing.validate()?;

        let mut tx = self.calendar.begin().await.map_err(storage_error)?;
        let user_id = command.user_id;
        let existing = tx
            .get_event_by_uid(user_id, &command.uid)
            .await
            .map_err(storage_error)?;

        if let (Some(event), Some(expected_etag)) = (&existing, &command.expected_etag) {
            let current_etag = format!("\"{}\"", event.etag);
            if expected_etag != "*" && expected_etag != &current_etag {
                return Err(ApplicationError::Conflict(format!(
                    "ETag mismatch: {} != {}",
                    expected_etag, current_etag
                )));
            }
        }

        let created = existing.is_none();
        let version = existing.as_ref().map_or(1, |event| event.version + 1);
        let sync_version = tx
            .bump_calendar_state(user_id)
            .await
            .map_err(storage_error)?;

        let provisional_etag = "pending".to_string();
        let event = if let Some(existing_event) = existing {
            tx.update_event(StoredEventUpdate {
                id: existing_event.id,
                user_id,
                summary: command.summary.clone(),
                description: command.description.clone(),
                location: command.location.clone(),
                timing: command.timing.clone(),
                status: command.status,
                rrule: command.rrule.clone(),
                version,
                sync_version,
                etag: provisional_etag,
            })
            .await
            .map_err(storage_error)?
        } else {
            tx.insert_event(StoredEventWrite {
                user_id,
                uid: command.uid.clone(),
                summary: command.summary.clone(),
                description: command.description.clone(),
                location: command.location.clone(),
                timing: command.timing.clone(),
                status: command.status,
                rrule: command.rrule.clone(),
                version,
                sync_version,
                etag: provisional_etag,
            })
            .await
            .map_err(storage_error)?
        };

        let attendee_writes: Vec<_> = command
            .attendees
            .iter()
            .map(|attendee| AttendeeWrite {
                email: attendee.email.clone(),
                user_id: attendee.user_id.map(UserId::inner),
                role: attendee.role,
                status: attendee.status,
            })
            .collect();
        let upsert_results = tx
            .replace_attendees(event.id, &attendee_writes)
            .await
            .map_err(storage_error)?;
        let final_attendees = tx.list_attendees(event.id).await.map_err(storage_error)?;
        let etag = etag_for_parts(
            &command.uid,
            &command.summary,
            command.description.clone(),
            command.location.clone(),
            command.timing,
            command.status,
            command.rrule,
            version,
            &final_attendees,
        );
        let event = tx
            .set_event_sync_etag(event.id, user_id, version, sync_version, etag)
            .await
            .map_err(storage_error)?;

        let mut outbox = Vec::new();
        for upsert_result in upsert_results {
            if !upsert_result.is_new {
                continue;
            }
            if let Some(target_user_id) = upsert_result.user_id {
                outbox.push(OutboxPayload::InviteNotification(InviteNotification {
                    event_id: event.id,
                    target_user_id,
                }));
            } else if let Some(attendee) = command
                .attendees
                .iter()
                .find(|attendee| attendee.email == upsert_result.email)
            {
                outbox.push(OutboxPayload::ExternalEmailDeferred(
                    ExternalEmailDeferred {
                        recipient_email: attendee.email.clone(),
                        event_summary: event.summary.clone(),
                        reason: "External email delivery is disabled".to_string(),
                    },
                ));
            }
        }
        tx.queue_outbox(&outbox).await.map_err(storage_error)?;

        tx.commit().await.map_err(storage_error)?;
        Ok(PutEventResult {
            etag: event.etag,
            created,
        })
    }

    pub async fn delete_event_by_id(
        &self,
        user_id: UserId,
        event_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let mut tx = self.calendar.begin().await.map_err(storage_error)?;
        let sync_version = tx
            .bump_calendar_state(user_id)
            .await
            .map_err(storage_error)?;
        let deleted = tx
            .delete_event_by_id(user_id, event_id)
            .await
            .map_err(storage_error)?
            .ok_or_else(|| ApplicationError::NotFound(event_id.to_string()))?;
        tx.insert_tombstone(user_id, &deleted.uid, sync_version)
            .await
            .map_err(storage_error)?;
        tx.commit().await.map_err(storage_error)?;
        Ok(())
    }

    pub async fn delete_event_by_uid(
        &self,
        user_id: UserId,
        uid: &str,
        expected_etag: Option<String>,
    ) -> Result<(), ApplicationError> {
        let mut tx = self.calendar.begin().await.map_err(storage_error)?;
        let current = tx
            .get_event_by_uid(user_id, uid)
            .await
            .map_err(storage_error)?
            .ok_or_else(|| ApplicationError::NotFound(uid.to_string()))?;

        if let Some(expected_etag) = expected_etag {
            let current_etag = format!("\"{}\"", current.etag);
            if expected_etag != "*" && expected_etag != current_etag {
                return Err(ApplicationError::Conflict(format!(
                    "ETag mismatch: {} != {}",
                    expected_etag, current_etag
                )));
            }
        }

        let sync_version = tx
            .bump_calendar_state(user_id)
            .await
            .map_err(storage_error)?;
        let deleted = tx
            .delete_event_by_uid(user_id, uid)
            .await
            .map_err(storage_error)?
            .ok_or_else(|| ApplicationError::NotFound(uid.to_string()))?;
        tx.insert_tombstone(user_id, &deleted.uid, sync_version)
            .await
            .map_err(storage_error)?;
        tx.commit().await.map_err(storage_error)?;
        Ok(())
    }

    pub async fn invite_attendee(
        &self,
        command: InviteAttendeeCommand,
    ) -> Result<(), ApplicationError> {
        let mut tx = self.calendar.begin().await.map_err(storage_error)?;
        let organizer_user_id = command.organizer_user_id;
        let current = tx
            .get_event_by_id(organizer_user_id, command.event_id)
            .await
            .map_err(storage_error)?
            .ok_or_else(|| ApplicationError::NotFound(command.event_id.to_string()))?;

        let attendees = [AttendeeWrite {
            email: command.email.clone(),
            user_id: command.attendee_user_id.map(UserId::inner),
            role: command.role,
            status: ParticipationStatus::NeedsAction,
        }];
        let upsert_results = tx
            .upsert_attendees(current.id, &attendees)
            .await
            .map_err(storage_error)?;

        let version = current.version + 1;
        let sync_version = tx
            .bump_calendar_state(organizer_user_id)
            .await
            .map_err(storage_error)?;
        let final_attendees = tx.list_attendees(current.id).await.map_err(storage_error)?;
        let etag = etag_for_event(&current, version, &final_attendees)?;
        let event = tx
            .set_event_sync_etag(current.id, organizer_user_id, version, sync_version, etag)
            .await
            .map_err(storage_error)?;

        let is_new = upsert_results.iter().any(|result| result.is_new);
        if is_new {
            let payload = if let Some(attendee_user_id) = command.attendee_user_id {
                OutboxPayload::InviteNotification(InviteNotification {
                    event_id: event.id,
                    target_user_id: attendee_user_id.inner(),
                })
            } else {
                OutboxPayload::ExternalEmailDeferred(ExternalEmailDeferred {
                    recipient_email: command.email,
                    event_summary: event.summary.clone(),
                    reason: "External email delivery is disabled".to_string(),
                })
            };
            tx.queue_outbox(&[payload]).await.map_err(storage_error)?;
        }

        tx.commit().await.map_err(storage_error)?;
        Ok(())
    }

    pub async fn confirm_rsvp(&self, command: ConfirmRsvpCommand) -> Result<(), ApplicationError> {
        let mut tx = self.calendar.begin().await.map_err(storage_error)?;
        let updated = tx
            .update_attendee_status(
                command.event_id,
                command.attendee_user_id.inner(),
                command.status,
            )
            .await
            .map_err(storage_error)?;

        if !updated {
            return Err(ApplicationError::NotFound(command.event_id.to_string()));
        }

        let current = tx
            .get_event_by_id_any(command.event_id)
            .await
            .map_err(storage_error)?
            .ok_or_else(|| ApplicationError::NotFound(command.event_id.to_string()))?;
        let organizer_user_id = current.user_id;
        let version = current.version + 1;
        let sync_version = tx
            .bump_calendar_state(organizer_user_id)
            .await
            .map_err(storage_error)?;
        let attendees = tx.list_attendees(current.id).await.map_err(storage_error)?;
        let etag = etag_for_event(&current, version, &attendees)?;
        let event = tx
            .set_event_sync_etag(current.id, organizer_user_id, version, sync_version, etag)
            .await
            .map_err(storage_error)?;

        tx.queue_outbox(&[OutboxPayload::RsvpNotification(RsvpNotification {
            organizer_telegram_id: organizer_user_id.inner(),
            attendee_name: command.attendee_name,
            event_summary: event.summary,
            rsvp_status: command.status,
        })])
        .await
        .map_err(storage_error)?;

        tx.commit().await.map_err(storage_error)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CreateEventCommand {
    pub user_id: UserId,
    pub username: Option<String>,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub timing: EventTiming,
    pub status: EventStatus,
    pub rrule: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateEventCommand {
    pub user_id: UserId,
    pub event_id: Uuid,
    pub summary: Option<String>,
    pub description: Option<Option<String>>,
    pub location: Option<Option<String>>,
    pub timing: Option<EventTiming>,
    pub status: Option<EventStatus>,
    pub rrule: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct PutEventCommand {
    pub user_id: UserId,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub timing: EventTiming,
    pub status: EventStatus,
    pub rrule: Option<String>,
    pub expected_etag: Option<String>,
    pub attendees: Vec<AttendeeCommand>,
}

#[derive(Debug, Clone)]
pub struct PutEventResult {
    pub etag: String,
    pub created: bool,
}

#[derive(Debug, Clone)]
pub struct AttendeeCommand {
    pub email: String,
    pub user_id: Option<UserId>,
    pub role: AttendeeRole,
    pub status: ParticipationStatus,
}

#[derive(Debug, Clone)]
pub struct InviteAttendeeCommand {
    pub organizer_user_id: UserId,
    pub event_id: Uuid,
    pub email: String,
    pub attendee_user_id: Option<UserId>,
    pub role: AttendeeRole,
}

#[derive(Debug, Clone)]
pub struct ConfirmRsvpCommand {
    pub event_id: Uuid,
    pub attendee_user_id: UserId,
    pub status: ParticipationStatus,
    pub attendee_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingInviteView {
    pub event_id: Uuid,
    pub summary: String,
    pub start: Option<DateTime<Utc>>,
    pub start_date: Option<NaiveDate>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub organizer_username: Option<String>,
}

impl From<PendingInviteRecord> for PendingInviteView {
    fn from(invite: PendingInviteRecord) -> Self {
        Self {
            event_id: invite.event_id,
            summary: invite.summary,
            start: invite.start,
            start_date: invite.start_date,
            is_all_day: invite.is_all_day,
            location: invite.location,
            organizer_username: invite.organizer_username,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttendeeDisplayView {
    pub email: String,
    pub telegram_id: Option<i64>,
    pub role: AttendeeRole,
    pub status: ParticipationStatus,
    pub telegram_username: Option<String>,
}

impl TryFrom<AttendeeDisplayRecord> for AttendeeDisplayView {
    type Error = ApplicationError;

    fn try_from(attendee: AttendeeDisplayRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            email: attendee.email,
            telegram_id: attendee.telegram_id,
            role: AttendeeRole::parse(&attendee.role).ok_or_else(|| {
                ApplicationError::Internal(format!("Unknown attendee role: {}", attendee.role))
            })?,
            status: ParticipationStatus::parse(&attendee.status).ok_or_else(|| {
                ApplicationError::Internal(format!(
                    "Unknown participation status: {}",
                    attendee.status
                ))
            })?,
            telegram_username: attendee.telegram_username,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventView {
    pub id: Uuid,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub timing: EventTiming,
    pub status: EventStatus,
    pub rrule: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserIdentity {
    pub id: UserId,
    pub username: Option<String>,
    pub timezone: Timezone,
}

impl From<User> for UserIdentity {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.telegram_username,
            timezone: user.timezone,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEventIcal {
    pub etag: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarIcalExport {
    pub event_count: usize,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalDavCalendarState {
    pub sync_token: i64,
    pub ctag: i64,
}

impl From<&User> for CalDavCalendarState {
    fn from(user: &User) -> Self {
        Self {
            sync_token: user.sync_token,
            ctag: user.ctag,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalDavUser {
    pub id: UserId,
    pub calendar: CalDavCalendarState,
}

impl From<User> for CalDavUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            calendar: CalDavCalendarState::from(&user),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalDavEventMetadata {
    pub uid: String,
    pub etag: String,
    pub updated_at: DateTime<Utc>,
}

impl From<&Event> for CalDavEventMetadata {
    fn from(event: &Event) -> Self {
        Self {
            uid: event.uid.clone(),
            etag: event.etag.clone(),
            updated_at: event.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalDavEventResource {
    pub uid: String,
    pub etag: String,
    pub updated_at: DateTime<Utc>,
    pub calendar_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalDavTombstone {
    pub uid: String,
}

impl From<EventTombstone> for CalDavTombstone {
    fn from(tombstone: EventTombstone) -> Self {
        Self { uid: tombstone.uid }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalDavSyncChanges {
    pub last_sync_token: i64,
    pub events: Vec<CalDavEventResource>,
    pub tombstones: Vec<CalDavTombstone>,
}

impl TryFrom<Event> for EventView {
    type Error = ApplicationError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        let timing = timing_from_event(&event)?;
        let status = event.status;

        Ok(Self {
            id: event.id,
            uid: event.uid,
            summary: event.summary,
            description: event.description,
            location: event.location,
            timing,
            status,
            rrule: event.rrule,
        })
    }
}

#[derive(Debug, Clone)]
struct CalendarEventsWithAttendees {
    events: Vec<Event>,
    attendees_by_event: HashMap<Uuid, Vec<EventAttendee>>,
}

#[derive(Debug, Clone)]
struct CalendarSyncChanges {
    last_sync_token: i64,
    events: Vec<Event>,
    tombstones: Vec<EventTombstone>,
    attendees_by_event: HashMap<Uuid, Vec<EventAttendee>>,
}

fn render_caldav_event_resources(
    events: Vec<Event>,
    attendees_by_event: &HashMap<Uuid, Vec<EventAttendee>>,
) -> Result<Vec<CalDavEventResource>, ApplicationError> {
    events
        .into_iter()
        .map(|event| render_caldav_event_resource(event, attendees_by_event))
        .collect()
}

fn render_caldav_event_resources_lossy(
    events: Vec<Event>,
    attendees_by_event: &HashMap<Uuid, Vec<EventAttendee>>,
) -> Vec<CalDavEventResource> {
    events
        .into_iter()
        .filter_map(|event| render_caldav_event_resource(event, attendees_by_event).ok())
        .collect()
}

fn render_caldav_event_resource(
    event: Event,
    attendees_by_event: &HashMap<Uuid, Vec<EventAttendee>>,
) -> Result<CalDavEventResource, ApplicationError> {
    let attendees = attendees_by_event
        .get(&event.id)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut calendar_data = String::with_capacity(1024);
    crate::ical::event_to_ical_into(
        &ical_event_render_from_event(&event)?,
        &ical_attendees(attendees),
        &mut calendar_data,
    )?;

    Ok(CalDavEventResource {
        uid: event.uid,
        etag: event.etag,
        updated_at: event.updated_at,
        calendar_data,
    })
}

fn ical_event_render_from_event(
    event: &Event,
) -> Result<crate::ical::IcalEventRender, ApplicationError> {
    Ok(crate::ical::IcalEventRender {
        uid: event.uid.clone(),
        summary: event.summary.clone(),
        description: event.description.clone(),
        location: event.location.clone(),
        timing: timing_from_event(event)?,
        status: event.status,
        rrule: event.rrule.clone(),
        sequence: event.version,
        created_at: event.created_at,
        updated_at: event.updated_at,
    })
}

fn ical_attendees(attendees: &[EventAttendee]) -> Vec<crate::ical::IcalAttendeeRender> {
    attendees
        .iter()
        .map(|attendee| crate::ical::IcalAttendeeRender {
            email: attendee.email.clone(),
            status: attendee.status,
        })
        .collect()
}

#[must_use]
pub fn parse_calendar_sync_token(sync_token: Option<&str>) -> i64 {
    sync_token
        .and_then(|token| token.rsplit('/').next())
        .and_then(|token| token.parse::<i64>().ok())
        .filter(|token| *token > 0)
        .unwrap_or(0)
}

fn timing_from_event(event: &Event) -> Result<EventTiming, ApplicationError> {
    if event.is_all_day {
        Ok(EventTiming::AllDay {
            start_date: event.start_date.ok_or_else(|| {
                ApplicationError::BadRequest("all-day event is missing start_date".to_string())
            })?,
            end_date: event.end_date.ok_or_else(|| {
                ApplicationError::BadRequest("all-day event is missing end_date".to_string())
            })?,
        })
    } else {
        Ok(EventTiming::Timed {
            start: event.start.ok_or_else(|| {
                ApplicationError::BadRequest("timed event is missing start".to_string())
            })?,
            end: event.end.ok_or_else(|| {
                ApplicationError::BadRequest("timed event is missing end".to_string())
            })?,
            timezone: event.timezone.clone(),
        })
    }
}

fn etag_for_event(
    event: &Event,
    version: i32,
    attendees: &[EventAttendee],
) -> Result<String, ApplicationError> {
    Ok(etag_for_parts(
        &event.uid,
        &event.summary,
        event.description.clone(),
        event.location.clone(),
        timing_from_event(event)?,
        event.status,
        event.rrule.clone(),
        version,
        attendees,
    ))
}

#[allow(clippy::too_many_arguments)]
fn etag_for_parts(
    uid: &str,
    summary: &str,
    description: Option<String>,
    location: Option<String>,
    timing: EventTiming,
    status: EventStatus,
    rrule: Option<String>,
    version: i32,
    attendees: &[EventAttendee],
) -> String {
    compute_event_etag(&EventEtagInput {
        uid: uid.to_string(),
        summary: summary.to_string(),
        description,
        location,
        timing,
        status,
        rrule,
        version,
        attendees: attendees
            .iter()
            .map(|attendee| AttendeeFingerprint {
                email: attendee.email.clone(),
                user_id: attendee.user_id,
                role: attendee.role,
                status: attendee.status,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_calendar_sync_token;

    #[test]
    fn parse_calendar_sync_token_accepts_caldav_url() {
        assert_eq!(
            parse_calendar_sync_token(Some("http://televent.app/sync/42")),
            42
        );
    }

    #[test]
    fn parse_calendar_sync_token_accepts_raw_number() {
        assert_eq!(parse_calendar_sync_token(Some("7")), 7);
    }

    #[test]
    fn parse_calendar_sync_token_defaults_for_initial_or_invalid_tokens() {
        assert_eq!(parse_calendar_sync_token(None), 0);
        assert_eq!(parse_calendar_sync_token(Some("")), 0);
        assert_eq!(
            parse_calendar_sync_token(Some("http://televent.app/sync/nope")),
            0
        );
        assert_eq!(
            parse_calendar_sync_token(Some("http://televent.app/sync/-1")),
            0
        );
    }
}
