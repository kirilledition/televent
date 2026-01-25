//! Event REST API endpoints

use crate::{db, error::ApiError, middleware::telegram_auth::AuthenticatedTelegramUser};
use axum::{
    Extension, Json, Router,
    extract::{FromRef, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use televent_core::models::{Event, EventStatus, Timezone, UserId};
use utoipa::ToSchema;
use uuid::Uuid;

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
    /// Start time (for timed events)
    pub start: DateTime<Utc>,
    /// End time (for timed events)
    pub end: DateTime<Utc>,
    /// Whether this is an all-day event
    pub is_all_day: bool,
    /// IANA timezone name
    pub timezone: Timezone,
    /// RFC 5545 recurrence rule
    #[schema(example = "FREQ=WEEKLY;BYDAY=MO")]
    pub rrule: Option<String>,
}

/// Update event request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEventRequest {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub is_all_day: Option<bool>,
    pub status: Option<EventStatus>,
    pub rrule: Option<String>,
}

/// List events query parameters
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
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

/// Event response (same as Event model)
#[derive(Debug, Serialize, ToSchema)]
pub struct EventResponse {
    pub id: Uuid,
    #[schema(value_type = String)]
    pub user_id: UserId,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    #[schema(value_type = Option<String>)]
    pub start_date: Option<chrono::NaiveDate>,
    #[schema(value_type = Option<String>)]
    pub end_date: Option<chrono::NaiveDate>,
    pub is_all_day: bool,
    pub status: EventStatus,
    pub timezone: Timezone,
    pub rrule: Option<String>,
    pub version: i32,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Event> for EventResponse {
    fn from(event: Event) -> Self {
        Self {
            id: event.id,
            user_id: event.user_id,
            uid: event.uid,
            summary: event.summary,
            description: event.description,
            location: event.location,
            start: event.start,
            end: event.end,
            start_date: event.start_date,
            end_date: event.end_date,
            is_all_day: event.is_all_day,
            status: event.status,
            timezone: event.timezone,
            rrule: event.rrule,
            version: event.version,
            etag: event.etag,
            created_at: event.created_at,
            updated_at: event.updated_at,
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
    State(pool): State<PgPool>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Json(req): Json<CreateEventRequest>,
) -> Result<Response, ApiError> {
    use crate::db::events::EventTiming;

    let timing = if req.is_all_day {
        EventTiming::AllDay {
            date: req.start.date_naive(),
            end_date: req.end.date_naive(),
        }
    } else {
        EventTiming::Timed {
            start: req.start,
            end: req.end,
        }
    };

    let event = db::events::create_event(
        &pool,
        auth_user.id,
        req.uid,
        req.summary,
        req.description,
        req.location,
        timing,
        req.timezone,
        req.rrule,
    )
    .await?;

    let response = EventResponse::from(event);
    Ok((StatusCode::CREATED, Json(response)).into_response())
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
    State(pool): State<PgPool>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventResponse>, ApiError> {
    let event = db::events::get_event(&pool, event_id).await?;
    Ok(Json(EventResponse::from(event)))
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
    State(pool): State<PgPool>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<Vec<EventResponse>>, ApiError> {
    // Default limit to 100 to prevent OOM
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    let events = db::events::list_events(
        &pool,
        auth_user.id,
        query.start,
        query.end,
        Some(limit),
        Some(offset),
    )
    .await?;
    let response = events.into_iter().map(EventResponse::from).collect();
    Ok(Json(response))
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
    State(pool): State<PgPool>,
    Path(event_id): Path<Uuid>,
    Json(req): Json<UpdateEventRequest>,
) -> Result<Json<EventResponse>, ApiError> {
    // Get current event to determine how to set date fields
    let current = db::events::get_event(&pool, event_id).await?;
    let is_all_day = req.is_all_day.unwrap_or(current.is_all_day);

    // Determine start_date/end_date based on is_all_day
    let (start, end, start_date, end_date) = if is_all_day {
        // For all-day events, use date fields
        let sd = req.start.map(|s| s.date_naive()).or(current.start_date);
        let ed = req.end.map(|e| e.date_naive()).or(current.end_date);
        (None, None, sd, ed)
    } else {
        // For timed events, use time fields
        let s = req.start.or(current.start);
        let e = req.end.or(current.end);
        (s, e, None, None)
    };

    let event = db::events::update_event(
        &pool,
        event_id,
        req.summary,
        req.description,
        req.location,
        start,
        end,
        start_date,
        end_date,
        req.is_all_day,
        req.status,
        req.rrule,
    )
    .await?;
    Ok(Json(EventResponse::from(event)))
}

/// Delete event
#[utoipa::path(
    delete,
    path = "/events/{id}",
    responses(
        (status = 201, description = "Event deleted successfully"),
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
    State(pool): State<PgPool>,
    Path(event_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    db::events::delete_event(&pool, event_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Event routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    PgPool: FromRef<S>,
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
    fn test_event_response_from_event() {
        use televent_core::models::Timezone;

        let event = Event {
            id: Uuid::new_v4(),
            user_id: UserId::new(123456789),
            uid: "test-uid".to_string(),
            summary: "Test Event".to_string(),
            description: Some("Description".to_string()),
            location: Some("Location".to_string()),
            start: Some(Utc::now()),
            end: Some(Utc::now()),
            start_date: None,
            end_date: None,
            is_all_day: false,
            status: EventStatus::Confirmed,
            timezone: Timezone::default(),
            rrule: None,
            version: 1,
            etag: "abc123".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response = EventResponse::from(event.clone());

        assert_eq!(response.id, event.id);
        assert_eq!(response.summary, event.summary);
        assert_eq!(response.status, event.status);
        assert_eq!(response.version, event.version);
    }

    #[test]
    fn test_create_event_request_deserialization() {
        let json = r#"{
            "uid": "test-uid",
            "summary": "Test Event",
            "description": "Test Description",
            "location": "Test Location",
            "start": "2026-01-18T10:00:00Z",
            "end": "2026-01-18T11:00:00Z",
            "is_all_day": false,
            "timezone": "UTC",
            "rrule": null
        }"#;

        let req: CreateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.summary, "Test Event");
        assert_eq!(req.timezone.to_string(), "UTC");
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
        assert!(req.start.is_none());
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
}
