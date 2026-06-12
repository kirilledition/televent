//! Event REST API endpoints

use crate::{error::ApiError, middleware::telegram_auth::AuthenticatedTelegramUser};
use axum::{
    Extension, Json, Router,
    extract::{FromRef, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use televent_application::{CalendarService, CreateEventCommand, EventView, UpdateEventCommand};
use televent_domain::{
    EventStatus as DomainEventStatus, EventTiming, MAX_DESCRIPTION_LENGTH, MAX_LOCATION_LENGTH,
    MAX_RRULE_LENGTH, MAX_SUMMARY_LENGTH, MAX_UID_LENGTH, Timezone, validate_length,
    validate_no_control_chars, validate_rrule, validate_safe_multiline_text,
};
use utoipa::ToSchema;
use uuid::Uuid;

// Constants for input validation
const MAX_EVENTS_LIMIT: i64 = 1000;

/// Create event request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEventRequest {
    /// iCalendar UID (stable across syncs)
    #[schema(example = "unique-uid-123")]
    pub uid: String,
    /// Event summary/title
    #[schema(example = "Team Meeting")]
    pub summary: String,
    /// Detailed description
    pub description: Option<String>,
    /// Event location
    pub location: Option<String>,
    /// Event timing discriminator
    pub timing: EventTimingRequest,
    /// RFC 5545 recurrence rule
    #[schema(example = "FREQ=WEEKLY;BYDAY=MO")]
    pub rrule: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventTimingRequest {
    Timed {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        timezone: String,
    },
    AllDay {
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
}

impl EventTimingRequest {
    fn into_domain(self) -> Result<EventTiming, ApiError> {
        match self {
            Self::Timed {
                start,
                end,
                timezone,
            } => Ok(EventTiming::Timed {
                start,
                end,
                timezone: Timezone::parse(timezone)
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?,
            }),
            Self::AllDay {
                start_date,
                end_date,
            } => Ok(EventTiming::AllDay {
                start_date,
                end_date,
            }),
        }
    }
}

impl CreateEventRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        validate_length("UID", &self.uid, MAX_UID_LENGTH).map_err(ApiError::BadRequest)?;
        validate_no_control_chars("UID", &self.uid).map_err(ApiError::BadRequest)?;

        validate_length("Summary", &self.summary, MAX_SUMMARY_LENGTH)
            .map_err(ApiError::BadRequest)?;
        validate_no_control_chars("Summary", &self.summary).map_err(ApiError::BadRequest)?;

        if let Some(description) = &self.description {
            validate_length("Description", description, MAX_DESCRIPTION_LENGTH)
                .map_err(ApiError::BadRequest)?;
            validate_safe_multiline_text("Description", description)
                .map_err(ApiError::BadRequest)?;
        }

        if let Some(location) = &self.location {
            validate_length("Location", location, MAX_LOCATION_LENGTH)
                .map_err(ApiError::BadRequest)?;
            validate_no_control_chars("Location", location).map_err(ApiError::BadRequest)?;
        }

        if let Some(rrule) = &self.rrule {
            validate_length("RRule", rrule, MAX_RRULE_LENGTH).map_err(ApiError::BadRequest)?;
            validate_no_control_chars("RRule", rrule).map_err(ApiError::BadRequest)?;
            validate_rrule(rrule).map_err(|err| ApiError::BadRequest(err.to_string()))?;
        }

        Ok(())
    }
}

/// Update event request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEventRequest {
    pub summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub location: Option<Option<String>>,
    pub timing: Option<EventTimingRequest>,
    pub status: Option<EventStatus>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub rrule: Option<Option<String>>,
}

impl UpdateEventRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        if let Some(summary) = &self.summary {
            validate_length("Summary", summary, MAX_SUMMARY_LENGTH)
                .map_err(ApiError::BadRequest)?;
            validate_no_control_chars("Summary", summary).map_err(ApiError::BadRequest)?;
        }

        if let Some(Some(description)) = &self.description {
            validate_length("Description", description, MAX_DESCRIPTION_LENGTH)
                .map_err(ApiError::BadRequest)?;
            validate_safe_multiline_text("Description", description)
                .map_err(ApiError::BadRequest)?;
        }

        if let Some(Some(location)) = &self.location {
            validate_length("Location", location, MAX_LOCATION_LENGTH)
                .map_err(ApiError::BadRequest)?;
            validate_no_control_chars("Location", location).map_err(ApiError::BadRequest)?;
        }

        if let Some(Some(rrule)) = &self.rrule {
            validate_length("RRule", rrule, MAX_RRULE_LENGTH).map_err(ApiError::BadRequest)?;
            validate_no_control_chars("RRule", rrule).map_err(ApiError::BadRequest)?;
            validate_rrule(rrule).map_err(|err| ApiError::BadRequest(err.to_string()))?;
        }

        Ok(())
    }
}

fn deserialize_nullable_update<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

impl EventStatus {
    fn into_domain(self) -> DomainEventStatus {
        match self {
            Self::Confirmed => DomainEventStatus::Confirmed,
            Self::Tentative => DomainEventStatus::Tentative,
            Self::Cancelled => DomainEventStatus::Cancelled,
        }
    }
}

impl From<DomainEventStatus> for EventStatus {
    fn from(value: DomainEventStatus) -> Self {
        match value {
            DomainEventStatus::Confirmed => Self::Confirmed,
            DomainEventStatus::Tentative => Self::Tentative,
            DomainEventStatus::Cancelled => Self::Cancelled,
        }
    }
}

/// List events query parameters
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListEventsQuery {
    /// Filter events starting after this time
    pub start: Option<DateTime<Utc>>,
    /// Filter events ending before this time
    pub end: Option<DateTime<Utc>>,
    /// Maximum number of events to return
    #[schema(default = 100)]
    pub limit: Option<i64>,
    /// Number of events to skip
    #[schema(default = 0)]
    pub offset: Option<i64>,
}

/// Public REST event response.
///
/// This intentionally hides storage/sync internals such as ETag, sync version,
/// DB timestamps, owner ID, and optimistic-locking version.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventResponse {
    pub id: Uuid,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub is_all_day: bool,
    pub status: EventStatus,
    pub timezone: String,
    pub rrule: Option<String>,
}

impl From<EventView> for EventResponse {
    fn from(event: EventView) -> Self {
        let (start, end, start_date, end_date, is_all_day, timezone) = match event.timing {
            EventTiming::Timed {
                start,
                end,
                timezone,
            } => (
                Some(start),
                Some(end),
                None,
                None,
                false,
                timezone.as_str().to_string(),
            ),
            EventTiming::AllDay {
                start_date,
                end_date,
            } => (
                None,
                None,
                Some(start_date),
                Some(end_date),
                true,
                "UTC".to_string(),
            ),
        };

        Self {
            id: event.id,
            uid: event.uid,
            summary: event.summary,
            description: event.description,
            location: event.location,
            start,
            end,
            start_date,
            end_date,
            is_all_day,
            status: event.status.into(),
            timezone,
            rrule: event.rrule,
        }
    }
}

/// Create a new event
#[utoipa::path(
    post,
    path = "/events",
    request_body = CreateEventRequest,
    responses(
        (status = 201, description = "Event created successfully", body = EventResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "events",
    security(
        ("telegram_auth" = [])
    )
)]
async fn create_event(
    State(calendar): State<CalendarService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Json(req): Json<CreateEventRequest>,
) -> Result<Response, ApiError> {
    req.validate()?;
    let event = calendar
        .create_event_view(CreateEventCommand {
            user_id: auth_user.id,
            username: auth_user.username,
            uid: req.uid,
            summary: req.summary,
            description: req.description,
            location: req.location,
            timing: req.timing.into_domain()?,
            status: DomainEventStatus::Confirmed,
            rrule: req.rrule,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(EventResponse::from(event))).into_response())
}

/// Get event by ID
#[utoipa::path(
    get,
    path = "/events/{id}",
    responses(
        (status = 200, description = "Event details", body = EventResponse),
        (status = 404, description = "Event not found"),
        (status = 401, description = "Unauthorized")
    ),
    params(
        ("id" = Uuid, Path, description = "Event ID")
    ),
    tag = "events",
    security(
        ("telegram_auth" = [])
    )
)]
async fn get_event(
    State(calendar): State<CalendarService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Path(event_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    // Check content negotiation
    if let Some(accept) = headers.get(header::ACCEPT)
        && let Ok(accept_str) = accept.to_str()
        && accept_str.contains("text/calendar")
    {
        let rendered = calendar.render_event_ical(auth_user.id, event_id).await?;

        return Ok((
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "text/calendar; charset=utf-8".to_string(),
            )],
            rendered.body,
        )
            .into_response());
    }

    let event = calendar.get_event_view(auth_user.id, event_id).await?;

    Ok(Json(EventResponse::from(event)).into_response())
}

/// List events
#[utoipa::path(
    get,
    path = "/events",
    params(ListEventsQuery),
    responses(
        (status = 200, description = "List of events", body = Vec<EventResponse>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "events",
    security(
        ("telegram_auth" = [])
    )
)]
async fn list_events(
    State(calendar): State<CalendarService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<Vec<EventResponse>>, ApiError> {
    // Default limit to 100 to prevent OOM
    let limit = query.limit.unwrap_or(100).clamp(1, MAX_EVENTS_LIMIT);
    let offset = query.offset.unwrap_or(0);

    let events = calendar
        .list_event_views(
            auth_user.id,
            query.start,
            query.end,
            Some(limit),
            Some(offset),
        )
        .await?;
    Ok(Json(events.into_iter().map(EventResponse::from).collect()))
}

/// Update event
#[utoipa::path(
    put,
    path = "/events/{id}",
    request_body = UpdateEventRequest,
    responses(
        (status = 200, description = "Event updated successfully", body = EventResponse),
        (status = 404, description = "Event not found"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    params(
        ("id" = Uuid, Path, description = "Event ID")
    ),
    tag = "events",
    security(
        ("telegram_auth" = [])
    )
)]
async fn update_event(
    State(calendar): State<CalendarService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Path(event_id): Path<Uuid>,
    Json(req): Json<UpdateEventRequest>,
) -> Result<Json<EventResponse>, ApiError> {
    req.validate()?;

    let event = calendar
        .update_event_view(UpdateEventCommand {
            user_id: auth_user.id,
            event_id,
            summary: req.summary,
            description: req.description,
            location: req.location,
            timing: req
                .timing
                .map(EventTimingRequest::into_domain)
                .transpose()?,
            status: req.status.map(EventStatus::into_domain),
            rrule: req.rrule,
        })
        .await?;
    Ok(Json(EventResponse::from(event)))
}

/// Delete event
#[utoipa::path(
    delete,
    path = "/events/{id}",
    responses(
        (status = 204, description = "Event deleted successfully"),
        (status = 404, description = "Event not found"),
        (status = 401, description = "Unauthorized")
    ),
    params(
        ("id" = Uuid, Path, description = "Event ID")
    ),
    tag = "events",
    security(
        ("telegram_auth" = [])
    )
)]
async fn delete_event_handler(
    State(calendar): State<CalendarService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Path(event_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    calendar.delete_event_by_id(auth_user.id, event_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Event routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    CalendarService: FromRef<S>,
{
    Router::new()
        .route("/events", post(create_event))
        .route("/events", get(list_events))
        .route("/events/{id}", get(get_event))
        .route("/events/{id}", put(update_event))
        .route("/events/{id}", delete(delete_event_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_event_request_deserialization() {
        let json = r#"{
            "uid": "test-uid",
            "summary": "Test Event",
            "description": "Test Description",
            "location": "Test Location",
            "timing": {
                "kind": "timed",
                "start": "2026-01-18T10:00:00Z",
                "end": "2026-01-18T11:00:00Z",
                "timezone": "UTC"
            },
            "rrule": null
        }"#;

        let req: CreateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.summary, "Test Event");
        assert!(matches!(req.timing, EventTimingRequest::Timed { .. }));
        assert_eq!(req.description, Some("Test Description".to_string()));
        assert!(req.rrule.is_none());
    }

    #[test]
    fn test_update_event_request_partial() {
        let json = r#"{
            "summary": "Updated Summary"
        }"#;

        let req: UpdateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.summary, Some("Updated Summary".to_string()));
        assert!(req.description.is_none());
        assert!(req.timing.is_none());
    }

    #[test]
    fn test_update_event_request_nullable_fields_distinguish_null_from_missing() {
        let json = r#"{
            "description": null,
            "location": "Room 1",
            "rrule": null
        }"#;

        let req: UpdateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.description, Some(None));
        assert_eq!(req.location, Some(Some("Room 1".to_string())));
        assert_eq!(req.rrule, Some(None));
        assert!(req.summary.is_none());
    }

    #[test]
    fn test_list_events_query_deserialization() {
        let json = r#"{
            "start": "2026-01-18T10:00:00Z",
            "limit": 50,
            "offset": 10
        }"#;

        let query: ListEventsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(10));
        assert!(query.start.is_some());
        assert!(query.end.is_none());
    }

    #[test]
    fn test_list_events_query_default_values() {
        let json = r#"{}"#;

        let query: ListEventsQuery = serde_json::from_str(json).unwrap();
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_create_event_validation_success() {
        let req = CreateEventRequest {
            uid: "valid-uid".to_string(),
            summary: "Valid Summary".to_string(),
            description: Some("Valid Description".to_string()),
            location: Some("Valid Location".to_string()),
            timing: EventTimingRequest::Timed {
                start: Utc::now(),
                end: Utc::now(),
                timezone: "UTC".to_string(),
            },
            rrule: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_event_validation_too_long() {
        let req = CreateEventRequest {
            uid: "a".repeat(MAX_UID_LENGTH + 1),
            summary: "Valid Summary".to_string(),
            description: None,
            location: None,
            timing: EventTimingRequest::Timed {
                start: Utc::now(),
                end: Utc::now(),
                timezone: "UTC".to_string(),
            },
            rrule: None,
        };
        assert!(req.validate().is_err());

        let req = CreateEventRequest {
            uid: "valid-uid".to_string(),
            summary: "a".repeat(MAX_SUMMARY_LENGTH + 1),
            description: None,
            location: None,
            timing: EventTimingRequest::Timed {
                start: Utc::now(),
                end: Utc::now(),
                timezone: "UTC".to_string(),
            },
            rrule: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_event_validation_control_chars() {
        let req = CreateEventRequest {
            uid: "valid-uid".to_string(),
            summary: "Invalid\nSummary".to_string(),
            description: None,
            location: None,
            timing: EventTimingRequest::Timed {
                start: Utc::now(),
                end: Utc::now(),
                timezone: "UTC".to_string(),
            },
            rrule: None,
        };
        assert!(req.validate().is_err());

        let req = CreateEventRequest {
            uid: "valid-uid".to_string(),
            summary: "Valid Summary".to_string(),
            description: Some("Valid\nDescription".to_string()),
            location: None,
            timing: EventTimingRequest::Timed {
                start: Utc::now(),
                end: Utc::now(),
                timezone: "UTC".to_string(),
            },
            rrule: None,
        };
        assert!(req.validate().is_ok());

        let req = CreateEventRequest {
            uid: "valid-uid".to_string(),
            summary: "Valid Summary".to_string(),
            description: Some("Invalid\x07Description".to_string()),
            location: None,
            timing: EventTimingRequest::Timed {
                start: Utc::now(),
                end: Utc::now(),
                timezone: "UTC".to_string(),
            },
            rrule: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_event_validation() {
        let req = UpdateEventRequest {
            summary: Some("a".repeat(MAX_SUMMARY_LENGTH + 1)),
            description: None,
            location: None,
            timing: None,
            status: None,
            rrule: None,
        };
        assert!(req.validate().is_err());

        let req = UpdateEventRequest {
            summary: Some("Valid".to_string()),
            description: None,
            location: None,
            timing: None,
            status: None,
            rrule: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_event_response_hides_storage_fields() {
        let now = Utc::now();
        let event = EventView {
            id: Uuid::new_v4(),
            uid: "uid-1".to_string(),
            summary: "Public Event".to_string(),
            description: None,
            location: None,
            timing: EventTiming::Timed {
                start: now,
                end: now + chrono::Duration::hours(1),
                timezone: Timezone::utc(),
            },
            status: DomainEventStatus::Confirmed,
            rrule: None,
        };

        let value = serde_json::to_value(EventResponse::from(event)).unwrap();
        let object = value.as_object().unwrap();

        for hidden_field in [
            "user_id",
            "version",
            "sync_version",
            "etag",
            "created_at",
            "updated_at",
        ] {
            assert!(
                !object.contains_key(hidden_field),
                "{hidden_field} leaked in EventResponse"
            );
        }
    }

    #[test]
    fn test_create_event_validation_rrule_injection() {
        let req = CreateEventRequest {
            uid: "valid-uid".to_string(),
            summary: "Valid Summary".to_string(),
            description: None,
            location: None,
            timing: EventTimingRequest::Timed {
                start: Utc::now(),
                end: Utc::now(),
                timezone: "UTC".to_string(),
            },
            rrule: Some("FREQ=DAILY\r\nATTENDEE:EVIL".to_string()),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_event_validation_invalid_rrule() {
        let req = CreateEventRequest {
            uid: "valid-uid".to_string(),
            summary: "Valid Summary".to_string(),
            description: None,
            location: None,
            timing: EventTimingRequest::Timed {
                start: Utc::now(),
                end: Utc::now(),
                timezone: "UTC".to_string(),
            },
            rrule: Some("INVALID=TRUE".to_string()),
        };
        assert!(req.validate().is_err());
    }
}
