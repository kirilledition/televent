//! Event REST API endpoints

use crate::{db, error::ApiError};
use axum::{
    Json, Router,
    extract::{FromRef, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use televent_core::models::{Event, EventStatus};
use televent_core::timezone::Timezone;
use uuid::Uuid;

/// Create event request
#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub calendar_id: Uuid,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub is_all_day: bool,
    pub timezone: Timezone,
    pub rrule: Option<String>,
}

/// Update event request
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
pub struct ListEventsQuery {
    pub calendar_id: Uuid,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Event response (same as Event model)
#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
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
            calendar_id: event.calendar_id,
            uid: event.uid,
            summary: event.summary,
            description: event.description,
            location: event.location,
            start: event.start,
            end: event.end,
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
async fn create_event(
    State(pool): State<PgPool>,
    Json(req): Json<CreateEventRequest>,
) -> Result<Response, ApiError> {
    let event = db::events::create_event(
        &pool,
        req.calendar_id,
        req.uid,
        req.summary,
        req.description,
        req.location,
        req.start,
        req.end,
        req.is_all_day,
        req.timezone,
        req.rrule,
    )
    .await?;

    let response = EventResponse::from(event);
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// Get event by ID
async fn get_event(
    State(pool): State<PgPool>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventResponse>, ApiError> {
    let event = db::events::get_event(&pool, event_id).await?;
    Ok(Json(EventResponse::from(event)))
}

/// List events
async fn list_events(
    State(pool): State<PgPool>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<Vec<EventResponse>>, ApiError> {
    // Default limit to 100 to prevent OOM
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    let events = db::events::list_events(
        &pool,
        query.calendar_id,
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
async fn update_event(
    State(pool): State<PgPool>,
    Path(event_id): Path<Uuid>,
    Json(req): Json<UpdateEventRequest>,
) -> Result<Json<EventResponse>, ApiError> {
    let event = db::events::update_event(
        &pool,
        event_id,
        req.summary,
        req.description,
        req.location,
        req.start,
        req.end,
        req.is_all_day,
        req.status,
        req.rrule,
    )
    .await?;
    Ok(Json(EventResponse::from(event)))
}

/// Delete event
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
        let event = Event {
            id: Uuid::new_v4(),
            calendar_id: Uuid::new_v4(),
            uid: "test-uid".to_string(),
            summary: "Test Event".to_string(),
            description: Some("Description".to_string()),
            location: Some("Location".to_string()),
            start: Utc::now(),
            end: Utc::now(),
            is_all_day: false,
            status: EventStatus::Confirmed,
            timezone: Timezone::new("UTC").unwrap(),
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
        assert_eq!(response.timezone.as_str(), "UTC");
    }

    #[test]
    fn test_create_event_request_deserialization() {
        let json = r#"{
            "calendar_id": "123e4567-e89b-12d3-a456-426614174000",
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
        assert_eq!(req.description, Some("Test Description".to_string()));
        assert!(req.rrule.is_none());
        assert_eq!(req.timezone.as_str(), "UTC");
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
            "calendar_id": "123e4567-e89b-12d3-a456-426614174000",
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
        let json = r#"{
            "calendar_id": "123e4567-e89b-12d3-a456-426614174000"
        }"#;

        let query: ListEventsQuery = serde_json::from_str(json).unwrap();
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }
}
