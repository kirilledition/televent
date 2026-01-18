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

use super::caldav_xml;

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
    State(_pool): State<PgPool>,
    Path((user_id, event_uid)): Path<(Uuid, String)>,
) -> Result<Response, ApiError> {
    // TODO: Look up event by UID
    // TODO: Convert to iCalendar format
    // TODO: Return with Content-Type: text/calendar

    tracing::debug!("GET event {} for user {}", event_uid, user_id);

    Err(ApiError::NotFound("Event not found".to_string()))
}

/// CalDAV PUT handler
///
/// Creates or updates an event from iCalendar data
async fn caldav_put_event(
    State(_pool): State<PgPool>,
    Path((user_id, event_uid)): Path<(Uuid, String)>,
    _body: Body,
) -> Result<Response, ApiError> {
    // TODO: Parse iCalendar from body
    // TODO: Create or update event in database
    // TODO: Return appropriate status code (201 Created or 204 No Content)

    tracing::debug!("PUT event {} for user {}", event_uid, user_id);

    Ok((StatusCode::NOT_IMPLEMENTED, "Not implemented yet").into_response())
}

/// CalDAV DELETE handler
///
/// Deletes an event
async fn caldav_delete_event(
    State(_pool): State<PgPool>,
    Path((user_id, event_uid)): Path<(Uuid, String)>,
    _headers: HeaderMap,
) -> Result<Response, ApiError> {
    // TODO: Check If-Match header for ETag
    // TODO: Delete event from database
    // TODO: Return 204 No Content on success

    tracing::debug!("DELETE event {} for user {}", event_uid, user_id);

    Ok((StatusCode::NOT_IMPLEMENTED, "Not implemented yet").into_response())
}

/// CalDAV routes
pub fn routes() -> Router<PgPool> {
    Router::new()
        // Calendar collection endpoints
        .route("/caldav/:user_id/", any(caldav_handler))
        // Event resource endpoints
        .route("/caldav/:user_id/:event_uid.ics", any(event_handler))
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
        Method::PUT => caldav_put_event(State(pool), Path((user_id, event_uid)), body).await,
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
