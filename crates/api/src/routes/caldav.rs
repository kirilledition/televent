//! CalDAV protocol endpoints
//!
//! Implements RFC 4791 (CalDAV), RFC 5545 (iCalendar), and RFC 6578 (sync-collection)

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderName, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::error::ApiError;

use super::{caldav_xml, ical};

/// CalDAV OPTIONS handler
///
/// Returns DAV capabilities and allowed methods
///
/// RFC 4791 Section 5.1: Calendar Access Support
async fn caldav_options() -> Response {
    (
        StatusCode::OK,
        [
            // DAV compliance classes
            (HeaderName::from_static("dav"), "1, calendar-access"),
            // Allowed HTTP methods
            (header::ALLOW, "OPTIONS, PROPFIND, REPORT, GET, PUT, DELETE"),
            // Calendar data types supported
            (HeaderName::from_static("cal-accessible"), "calendar"),
        ],
    )
        .into_response()
}

/// CalDAV PROPFIND handler
///
/// Returns calendar properties based on Depth header:
/// - Depth: 0 - Calendar metadata only
/// - Depth: 1 - Calendar metadata + event list (hrefs)
async fn caldav_propfind(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    headers: HeaderMap,
    _body: Body,
) -> Result<Response, ApiError> {
    // Get Depth header (default to 0)
    let depth = headers
        .get("Depth")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0");

    tracing::debug!("PROPFIND request for user {} with Depth: {}", user_id, depth);

    // Get or create calendar for user
    let calendar = db::calendars::get_or_create_calendar(&pool, user_id).await?;

    // Get events if depth is 1
    let events = if depth == "1" {
        db::events::list_events(&pool, calendar.id, None, None).await?
    } else {
        Vec::new()
    };

    // Generate XML response
    let response_xml = caldav_xml::generate_propfind_multistatus(user_id, &calendar, &events, depth)?;

    Ok((
        StatusCode::MULTI_STATUS,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        response_xml,
    )
        .into_response())
}

/// CalDAV GET handler
///
/// Returns a single event in iCalendar format
async fn caldav_get_event(
    State(pool): State<PgPool>,
    Path((user_id, event_uid)): Path<(Uuid, String)>,
) -> Result<Response, ApiError> {
    tracing::debug!("GET event {} for user {}", event_uid, user_id);

    // Get calendar for user
    let calendar = db::calendars::get_or_create_calendar(&pool, user_id).await?;

    // Look up event by UID
    let event = db::events::get_event_by_uid(&pool, calendar.id, &event_uid)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Event not found: {}", event_uid)))?;

    // Convert to iCalendar format
    let ical = ical::event_to_ical(&event)?;

    // Return with proper headers
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (header::ETAG, &format!("\"{}\"", event.etag)),
        ],
        ical,
    )
        .into_response())
}

/// Maximum allowed body size for CalDAV requests (1 MB)
const MAX_CALDAV_BODY_SIZE: usize = 1024 * 1024;

/// CalDAV PUT handler
///
/// Creates or updates an event from iCalendar data
async fn caldav_put_event(
    State(pool): State<PgPool>,
    Path((user_id, event_uid)): Path<(Uuid, String)>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ApiError> {
    tracing::debug!("PUT event {} for user {}", event_uid, user_id);

    // Get calendar for user
    let calendar = db::calendars::get_or_create_calendar(&pool, user_id).await?;

    // Read body as string with size limit to prevent DoS
    let body_bytes = axum::body::to_bytes(body, MAX_CALDAV_BODY_SIZE)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read body: {}", e)))?;
    let ical_str = String::from_utf8(body_bytes.to_vec())
        .map_err(|e| ApiError::BadRequest(format!("Invalid UTF-8: {}", e)))?;

    // Parse iCalendar
    let (uid, summary, description, location, start, end, is_all_day, rrule, status, timezone) =
        ical::ical_to_event_data(&ical_str)?;

    // Check if UID matches the URL
    if uid != event_uid {
        return Err(ApiError::BadRequest(format!(
            "UID mismatch: {} != {}",
            uid, event_uid
        )));
    }

    // Check if event already exists
    let existing = db::events::get_event_by_uid(&pool, calendar.id, &uid).await?;

    let (status_code, etag) = if let Some(existing_event) = existing {
        // Check If-Match header for optimistic locking (RFC 4791)
        if let Some(if_match) = headers.get(header::IF_MATCH) {
            let requested_etag = if_match
                .to_str()
                .map_err(|_| ApiError::BadRequest("Invalid If-Match header".to_string()))?;
            let current_etag = format!("\"{}\"", existing_event.etag);
            if requested_etag != current_etag && requested_etag != "*" {
                return Err(ApiError::Conflict(format!(
                    "ETag mismatch: {} != {}",
                    requested_etag, current_etag
                )));
            }
        }

        // Update existing event
        let updated = db::events::update_event(
            &pool,
            existing_event.id,
            Some(summary),
            description,
            location,
            Some(start),
            Some(end),
            Some(is_all_day),
            Some(status),
            rrule,
        )
        .await?;
        (StatusCode::NO_CONTENT, updated.etag)
    } else {
        // Create new event
        let created = db::events::create_event(
            &pool,
            calendar.id,
            uid,
            summary,
            description,
            location,
            start,
            end,
            is_all_day,
            timezone,
            rrule,
        )
        .await?;
        (StatusCode::CREATED, created.etag)
    };

    // Increment sync token
    let _new_sync_token = db::calendars::increment_sync_token(&pool, calendar.id).await?;

    Ok((
        status_code,
        [(header::ETAG, format!("\"{}\"", etag))],
        "",
    )
        .into_response())
}

/// CalDAV DELETE handler
///
/// Deletes an event
async fn caldav_delete_event(
    State(pool): State<PgPool>,
    Path((user_id, event_uid)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    tracing::debug!("DELETE event {} for user {}", event_uid, user_id);

    // Get calendar for user
    let calendar = db::calendars::get_or_create_calendar(&pool, user_id).await?;

    // Check If-Match header for ETag (optional but recommended)
    if let Some(if_match) = headers.get(header::IF_MATCH) {
        let requested_etag = if_match
            .to_str()
            .map_err(|_| ApiError::BadRequest("Invalid If-Match header".to_string()))?;

        // Get current event to check ETag
        if let Some(event) = db::events::get_event_by_uid(&pool, calendar.id, &event_uid).await? {
            let current_etag = format!("\"{}\"", event.etag);
            if requested_etag != current_etag && requested_etag != "*" {
                return Err(ApiError::Conflict(format!(
                    "ETag mismatch: {} != {}",
                    requested_etag, current_etag
                )));
            }
        }
    }

    // Delete event
    let deleted = db::events::delete_event_by_uid(&pool, calendar.id, &event_uid).await?;

    if !deleted {
        return Err(ApiError::NotFound(format!(
            "Event not found: {}",
            event_uid
        )));
    }

    // Increment sync token
    let _new_sync_token = db::calendars::increment_sync_token(&pool, calendar.id).await?;

    Ok((StatusCode::NO_CONTENT, "").into_response())
}

/// CalDAV routes
pub fn routes() -> Router<PgPool> {
    Router::new()
        // Calendar collection endpoints
        .route("/:user_id/", any(caldav_handler))
        // Event resource endpoints
        .route("/:user_id/:event_uid.ics", any(event_handler))
}

/// Main CalDAV collection handler
async fn caldav_handler(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    headers: HeaderMap,
    method: Method,
    body: Body,
) -> Result<Response, ApiError> {
    // Handle WebDAV methods
    if method == Method::OPTIONS {
        Ok(caldav_options().await)
    } else if method.as_str() == "PROPFIND" {
        caldav_propfind(State(pool), Path(user_id), headers, body).await
    } else {
        Err(ApiError::BadRequest(format!(
            "Method {} not supported for calendar collection",
            method
        )))
    }
}

/// Event resource handler
async fn event_handler(
    State(pool): State<PgPool>,
    Path((user_id, event_uid)): Path<(Uuid, String)>,
    headers: HeaderMap,
    method: Method,
    body: Body,
) -> Result<Response, ApiError> {
    match method {
        Method::GET => caldav_get_event(State(pool), Path((user_id, event_uid))).await,
        Method::PUT => {
            caldav_put_event(State(pool), Path((user_id, event_uid)), headers, body).await
        }
        Method::DELETE => {
            caldav_delete_event(State(pool), Path((user_id, event_uid)), headers).await
        }
        _ => Err(ApiError::BadRequest(format!(
            "Method {} not supported for event resource",
            method
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_caldav_options_response() {
        let response = caldav_options().await;
        let (parts, _body) = response.into_parts();

        assert_eq!(parts.status, StatusCode::OK);

        // Check DAV header
        let dav_header = parts.headers.get(&HeaderName::from_static("dav")).unwrap();
        assert_eq!(dav_header, "1, calendar-access");

        // Check ALLOW header
        let allow_header = parts.headers.get(header::ALLOW).unwrap();
        let allow_str = allow_header.to_str().unwrap();
        assert!(allow_str.contains("OPTIONS"));
        assert!(allow_str.contains("PROPFIND"));
        assert!(allow_str.contains("REPORT"));
        assert!(allow_str.contains("GET"));
        assert!(allow_str.contains("PUT"));
        assert!(allow_str.contains("DELETE"));
    }
}
