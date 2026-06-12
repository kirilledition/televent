//! CalDAV protocol endpoints
//!
//! Implements RFC 4791 (CalDAV), RFC 5545 (iCalendar), and RFC 6578 (sync-collection)

use std::borrow::Cow;

use axum::{
    Router,
    body::Body,
    extract::{Extension, FromRef, Path, State},
    http::{HeaderMap, HeaderName, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
};
use televent_application::{CalDavUser, CalendarService, UserId};

use crate::error::ApiError;
use crate::routes::{caldav_ical, caldav_xml};

/// CalDAV OPTIONS handler
///
/// Returns DAV capabilities and allowed methods
///
/// RFC 4791 Section 5.1: Calendar Access Support
async fn caldav_options() -> Response {
    tracing::info!("Handling CalDAV OPTIONS request");
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
    State(calendar): State<CalendarService>,
    Path(user_identifier): Path<String>,
    auth_user_id: UserId,
    headers: HeaderMap,
    _body: Body,
) -> Result<Response, ApiError> {
    let user = resolve_user(&calendar, &user_identifier).await?;

    if user.id != auth_user_id {
        return Err(ApiError::Forbidden);
    }

    // Get Depth header (default to 0)
    let depth = headers
        .get("Depth")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0");

    tracing::debug!(
        "PROPFIND request for user {} ({}) with Depth: {}",
        user_identifier,
        user.id,
        depth
    );

    // Get events if depth is 1
    let events = if depth == "1" {
        calendar
            .list_caldav_event_metadata(user.id, None, None)
            .await?
    } else {
        Vec::new()
    };

    // Generate XML response
    let response_xml = caldav_xml::generate_propfind_multistatus(
        &user_identifier,
        &user.calendar,
        &events,
        depth,
    )?;

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
    State(calendar): State<CalendarService>,
    Path((user_identifier, event_uid)): Path<(String, String)>,
    auth_user_id: UserId,
) -> Result<Response, ApiError> {
    let user = resolve_user(&calendar, &user_identifier).await?;

    if user.id != auth_user_id {
        return Err(ApiError::Forbidden);
    }

    tracing::debug!("GET event {} for user {}", event_uid, user.id);

    let rendered = calendar
        .render_event_ical_by_uid(user.id, &event_uid)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Event not found: {}", event_uid)))?;

    // Return with proper headers
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (header::ETAG, &format!("\"{}\"", rendered.etag)),
        ],
        rendered.body,
    )
        .into_response())
}

/// Maximum allowed body size for CalDAV requests (1 MB)
const MAX_CALDAV_BODY_SIZE: usize = 1024 * 1024;

/// Maximum number of hrefs allowed in a calendar-multiget report
const MAX_MULTIGET_HREFS: usize = 200;

/// CalDAV PUT handler
///
/// Creates or updates an event from iCalendar data
async fn caldav_put_event(
    State(calendar_service): State<CalendarService>,
    Path((user_identifier, event_uid)): Path<(String, String)>,
    auth_user_id: UserId,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ApiError> {
    let user = resolve_user(&calendar_service, &user_identifier).await?;

    if user.id != auth_user_id {
        return Err(ApiError::Forbidden);
    }

    tracing::debug!("PUT event {} for user {}", event_uid, user.id);

    // Read body as string with size limit to prevent DoS
    let body_bytes = axum::body::to_bytes(body, MAX_CALDAV_BODY_SIZE)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read body: {}", e)))?;
    let ical_str = String::from_utf8(body_bytes.to_vec())
        .map_err(|e| ApiError::BadRequest(format!("Invalid UTF-8: {}", e)))?;

    let parsed_event = caldav_ical::parse_put_event(&ical_str, &event_uid, user.id)?;

    let expected_etag = headers
        .get(header::IF_MATCH)
        .map(|value| {
            value
                .to_str()
                .map(str::to_string)
                .map_err(|_| ApiError::BadRequest("Invalid If-Match header".to_string()))
        })
        .transpose()?;

    let result = calendar_service
        .put_event_by_uid(parsed_event.into_put_command(user.id, expected_etag))
        .await?;
    let status_code = if result.created {
        StatusCode::CREATED
    } else {
        StatusCode::NO_CONTENT
    };
    let etag = result.etag;

    Ok((status_code, [(header::ETAG, format!("\"{}\"", etag))], "").into_response())
}

/// CalDAV REPORT handler
///
/// Handles calendar-query and sync-collection reports (RFC 4791, RFC 6578)
async fn caldav_report(
    State(calendar): State<CalendarService>,
    Path(user_identifier): Path<String>,
    auth_user_id: UserId,
    body: Body,
) -> Result<Response, ApiError> {
    let user = resolve_user(&calendar, &user_identifier).await?;

    if user.id != auth_user_id {
        return Err(ApiError::Forbidden);
    }

    tracing::debug!("REPORT request for user {}", user.id);

    // Read body
    let body_bytes = axum::body::to_bytes(body, MAX_CALDAV_BODY_SIZE)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read body: {}", e)))?;
    let xml_body = String::from_utf8(body_bytes.to_vec())
        .map_err(|e| ApiError::BadRequest(format!("Invalid UTF-8: {}", e)))?;

    tracing::debug!("REPORT XML body: {}", xml_body);

    // Parse report type
    let report_type = caldav_xml::parse_report_request(&xml_body).map_err(|e| {
        // Truncate XML body to prevent log flooding and sensitive data leakage
        let preview: String = xml_body.chars().take(256).collect();
        tracing::error!(
            "Failed to parse REPORT XML: {:?}\nXML body (truncated):\n{}",
            e,
            preview
        );
        e
    })?;

    match report_type {
        caldav_xml::ReportType::CalendarQuery { start, end } => {
            let events = calendar
                .list_caldav_event_resources(user.id, start, end)
                .await?;

            tracing::info!(
                "CalendarQuery: returning {} events (range: {:?} to {:?})",
                events.len(),
                start,
                end
            );

            let response_xml =
                caldav_xml::generate_calendar_query_response(&user_identifier, &events)?;

            tracing::debug!(
                "CalendarQuery response XML (first 500 chars): {}",
                &response_xml.chars().take(500).collect::<String>()
            );

            Ok((
                StatusCode::MULTI_STATUS,
                [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
                response_xml,
            )
                .into_response())
        }
        caldav_xml::ReportType::SyncCollection { sync_token } => {
            let resource_changes = calendar
                .list_caldav_sync_changes(user.id, sync_token.as_deref())
                .await?;

            tracing::info!(
                "SyncCollection: sync_token={:?}, parsed={}, returning {} events, user sync_token={}",
                sync_token,
                resource_changes.last_sync_token,
                resource_changes.events.len(),
                user.calendar.sync_token
            );

            let response_xml = caldav_xml::generate_sync_collection_response(
                &user_identifier,
                &user.calendar,
                &resource_changes.events,
                &resource_changes.tombstones,
            )?;

            // Log the actual XML response for debugging
            if !resource_changes.events.is_empty() {
                tracing::info!(
                    "SyncCollection XML response (first 2000 chars):\n{}",
                    &response_xml.chars().take(2000).collect::<String>()
                );
            }

            Ok((
                StatusCode::MULTI_STATUS,
                [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
                response_xml,
            )
                .into_response())
        }
        caldav_xml::ReportType::CalendarMultiget { hrefs } => {
            tracing::info!("CalendarMultiget: {} hrefs requested", hrefs.len());

            if hrefs.len() > MAX_MULTIGET_HREFS {
                return Err(ApiError::BadRequest(format!(
                    "Too many hrefs requested (max {})",
                    MAX_MULTIGET_HREFS
                )));
            }

            // Extract UIDs from hrefs
            // Format: /caldav/{user_id}/{uid}.ics or URL-encoded variants
            // Optimized to minimize allocations
            let requested_uids = extract_uids_from_hrefs(&hrefs);

            let uid_strs: Vec<&str> = requested_uids.iter().map(|s| s.as_ref()).collect();
            let events = calendar
                .list_caldav_event_resources_by_uids(user.id, &uid_strs)
                .await?;

            let response_xml =
                caldav_xml::generate_calendar_multiget_response(&user_identifier, &events)?;

            tracing::info!("CalendarMultiget: returning {} events", events.len());

            Ok((
                StatusCode::MULTI_STATUS,
                [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
                response_xml,
            )
                .into_response())
        }
    }
}

/// CalDAV DELETE handler
///
/// Deletes an event
async fn caldav_delete_event(
    State(calendar): State<CalendarService>,
    Path((user_identifier, event_uid)): Path<(String, String)>,
    auth_user_id: UserId,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let user = resolve_user(&calendar, &user_identifier).await?;

    if user.id != auth_user_id {
        return Err(ApiError::Forbidden);
    }

    tracing::debug!("DELETE event {} for user {}", event_uid, user.id);

    let expected_etag = headers
        .get(header::IF_MATCH)
        .map(|value| {
            value
                .to_str()
                .map(str::to_string)
                .map_err(|_| ApiError::BadRequest("Invalid If-Match header".to_string()))
        })
        .transpose()?;

    calendar
        .delete_event_by_uid(user.id, &event_uid, expected_etag)
        .await?;

    Ok((StatusCode::NO_CONTENT, "").into_response())
}

/// Resolve user identifier (numeric ID or username) to CalDAV user projection.
///
/// The identifier can be:
/// - A numeric Telegram ID (e.g., "123456789")
/// - A Telegram username (e.g., "myusername")
async fn resolve_user(
    calendar: &CalendarService,
    identifier: &str,
) -> Result<CalDavUser, ApiError> {
    calendar
        .resolve_caldav_user(identifier)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {identifier}")))
}

/// Helper to extract UIDs from hrefs optimally
///
/// Handles both raw and URL-encoded hrefs.
/// - For raw hrefs (no encoding), returns Cow::Borrowed slice (zero allocation).
/// - For encoded hrefs, reuses the allocation from decoding to store the UID (zero extra allocation).
fn extract_uids_from_hrefs(hrefs: &[String]) -> Vec<Cow<'_, str>> {
    let mut uids = Vec::with_capacity(hrefs.len());
    for href in hrefs {
        // 1. Extract the last path segment (raw) first to avoid decoding slashes that are part of the path structure
        let last_segment = href.rsplit('/').next().unwrap_or(href);

        // 2. Decode the segment
        let decoded = urlencoding::decode(last_segment).unwrap_or(Cow::Borrowed(last_segment));

        // 3. Trim .ics extension and push
        match decoded {
            Cow::Borrowed(s) => {
                let uid = s.trim_end_matches(".ics");
                if !uid.is_empty() {
                    uids.push(Cow::Borrowed(uid));
                }
            }
            Cow::Owned(mut s) => {
                // Optimization: reuse allocation from decoding
                if s.ends_with(".ics") {
                    let new_len = s.len() - 4;
                    s.truncate(new_len);
                }
                if !s.is_empty() {
                    uids.push(Cow::Owned(s));
                }
            }
        }
    }
    uids
}

/// CalDAV routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    CalendarService: FromRef<S>,
{
    Router::new()
        // Calendar collection endpoints
        .route("/{user_identifier}/", any(caldav_handler))
        // Event resource endpoints
        .route("/{user_identifier}/{*event_uid}", any(event_handler))
}

/// Main CalDAV collection handler
async fn caldav_handler(
    State(calendar): State<CalendarService>,
    Extension(auth_user_id): Extension<UserId>,
    Path(user_identifier): Path<String>,
    headers: HeaderMap,
    method: Method,
    body: Body,
) -> Result<Response, ApiError> {
    // Handle WebDAV methods
    match method.as_str() {
        "OPTIONS" => Ok(caldav_options().await),
        "PROPFIND" => {
            caldav_propfind(
                State(calendar),
                Path(user_identifier),
                auth_user_id,
                headers,
                body,
            )
            .await
        }
        "REPORT" => caldav_report(State(calendar), Path(user_identifier), auth_user_id, body).await,
        _ => Err(ApiError::BadRequest(format!(
            "Method {} not supported for calendar collection",
            method
        ))),
    }
}

/// Event resource handler
async fn event_handler(
    State(calendar): State<CalendarService>,
    Extension(auth_user_id): Extension<UserId>,
    Path((user_identifier, event_uid_raw)): Path<(String, String)>,
    headers: HeaderMap,
    method: Method,
    body: Body,
) -> Result<Response, ApiError> {
    // Strip leading slash and .ics extension from wildcard capture
    let event_uid = event_uid_raw
        .trim_start_matches('/')
        .trim_end_matches(".ics")
        .to_string();

    match method {
        Method::GET => {
            caldav_get_event(
                State(calendar),
                Path((user_identifier, event_uid)),
                auth_user_id,
            )
            .await
        }
        Method::PUT => {
            caldav_put_event(
                State(calendar),
                Path((user_identifier, event_uid)),
                auth_user_id,
                headers,
                body,
            )
            .await
        }
        Method::DELETE => {
            caldav_delete_event(
                State(calendar),
                Path((user_identifier, event_uid)),
                auth_user_id,
                headers,
            )
            .await
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
        let dav_header = parts.headers.get(HeaderName::from_static("dav")).unwrap();
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

    #[test]
    fn test_extract_uids_from_hrefs() {
        let hrefs = vec![
            "/caldav/user/uid1.ics".to_string(),
            "/caldav/user/uid2".to_string(),
            "uid3.ics".to_string(),
            "uid4".to_string(),
            "/caldav/user/uid%205.ics".to_string(), // encoded space
            "/caldav/user/uid%2F6.ics".to_string(), // encoded slash
        ];

        let uids = extract_uids_from_hrefs(&hrefs);

        assert_eq!(uids.len(), 6);
        assert_eq!(uids[0], "uid1");
        assert_eq!(uids[1], "uid2");
        assert_eq!(uids[2], "uid3");
        assert_eq!(uids[3], "uid4");
        assert_eq!(uids[4], "uid 5");
        assert_eq!(uids[5], "uid/6");

        // Verify borrowing behavior (optimization check)
        // uid1 should be borrowed from hrefs
        match uids[0] {
            Cow::Borrowed(_) => {} // OK
            Cow::Owned(_) => panic!("uid1 should be borrowed"),
        }

        // uid 5 should be owned (because of decoding)
        if let Cow::Borrowed(_) = uids[4] {
            panic!("uid 5 should be owned");
        }
    }
}
