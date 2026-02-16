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
use sqlx::PgPool;
use televent_core::models::{User, UserId};

use crate::db;
use crate::error::ApiError;
use crate::routes::{caldav_xml, ical as ical_route};
use televent_core::attendee;
use televent_core::models::ParticipationStatus;
use televent_core::validation::{
    MAX_DESCRIPTION_LENGTH, MAX_LOCATION_LENGTH, MAX_RRULE_LENGTH, MAX_SUMMARY_LENGTH,
    MAX_UID_LENGTH, validate_length, validate_no_control_chars,
};

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
    State(pool): State<PgPool>,
    Path(user_identifier): Path<String>,
    auth_user_id: UserId,
    headers: HeaderMap,
    _body: Body,
) -> Result<Response, ApiError> {
    let user = resolve_user(&pool, &user_identifier).await?;

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
        db::events::list_events(&pool, user.id, None, None, None, None).await?
    } else {
        Vec::new()
    };

    // Generate XML response
    let response_xml =
        caldav_xml::generate_propfind_multistatus(&user_identifier, &user, &events, depth)?;

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
    Path((user_identifier, event_uid)): Path<(String, String)>,
    auth_user_id: UserId,
) -> Result<Response, ApiError> {
    let user = resolve_user(&pool, &user_identifier).await?;

    if user.id != auth_user_id {
        return Err(ApiError::Forbidden);
    }

    tracing::debug!("GET event {} for user {}", event_uid, user.id);

    // Look up event by UID
    let event = db::events::get_event_by_uid(&pool, user.id, &event_uid)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Event not found: {}", event_uid)))?;

    // List attendees
    let attendees = db::events::get_event_attendees(&pool, event.id).await?;

    // Convert to iCalendar format
    let ical = ical_route::event_to_ical(&event, &attendees)?;

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

/// Maximum number of hrefs allowed in a calendar-multiget report
const MAX_MULTIGET_HREFS: usize = 200;

/// CalDAV PUT handler
///
/// Creates or updates an event from iCalendar data
/// CalDAV PUT handler
///
/// Creates or updates an event from iCalendar data
async fn caldav_put_event(
    State(pool): State<PgPool>,
    Path((user_identifier, event_uid)): Path<(String, String)>,
    auth_user_id: UserId,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ApiError> {
    let user = resolve_user(&pool, &user_identifier).await?;

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

    // Parse iCalendar using ical crate
    let parser = ical::IcalParser::new(std::io::Cursor::new(&ical_str));
    let calendar = parser
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::BadRequest("Empty calendar".to_string()))?
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse calendar: {}", e)))?;

    let event = calendar
        .events
        .first()
        .ok_or_else(|| ApiError::BadRequest("No event found in calendar".to_string()))?;

    // Extract basic properties
    // We'll use a local helper or inline logic. For now, let's reuse ical_to_event_data logic but adapted?
    // actually, let's just use ical_to_event_data for the basic fields to avoid rewriting parsing logic for dates/recurrence right now,
    // AND iterate over the parsed `event` properties for attendees.
    // This parses twice but is safer to avoid regression on date parsing which can be complex.
    // Wait, ical_to_event_data parses the STRING.
    // Efficiently, we should do it once.
    // But ical_to_event_data is robust for now.
    // Correct approach: Use `ical_to_event_data` for event fields, and `ical` crate for Attendees.
    // This is technically double parsing but negligible for small ICS files.
    //
    // TODO: Refactor ical_to_event_data to use ical crate internally later.

    let (uid, summary, description, location, start, end, is_all_day, rrule, status, timezone) =
        ical_route::ical_to_event_data(&ical_str)?;

    // Validate inputs
    validate_length("UID", &uid, MAX_UID_LENGTH).map_err(ApiError::BadRequest)?;
    validate_length("Summary", &summary, MAX_SUMMARY_LENGTH).map_err(ApiError::BadRequest)?;

    if let Some(desc) = &description {
        validate_length("Description", desc, MAX_DESCRIPTION_LENGTH)
            .map_err(ApiError::BadRequest)?;
    }

    if let Some(loc) = &location {
        validate_length("Location", loc, MAX_LOCATION_LENGTH).map_err(ApiError::BadRequest)?;
    }

    if let Some(r) = &rrule {
        validate_length("RRule", r, MAX_RRULE_LENGTH).map_err(ApiError::BadRequest)?;
        validate_no_control_chars("RRule", r).map_err(ApiError::BadRequest)?;
    }

    // Check if UID matches the URL
    if uid != event_uid {
        return Err(ApiError::BadRequest(format!(
            "UID mismatch: {} != {}",
            uid, event_uid
        )));
    }

    // Start transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Check if event already exists
    let existing = db::events::get_event_by_uid(&mut *tx, user.id, &uid).await?;

    let (status_code, etag, event_id) = if let Some(existing_event) = existing {
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
            &mut tx,
            user.id,
            existing_event.id,
            Some(summary),
            description,
            location,
            if is_all_day { None } else { Some(start) },
            if is_all_day { None } else { Some(end) },
            if is_all_day {
                Some(start.date_naive())
            } else {
                None
            },
            if is_all_day {
                Some(end.date_naive())
            } else {
                None
            },
            Some(is_all_day),
            Some(status),
            rrule,
        )
        .await?;
        (StatusCode::NO_CONTENT, updated.etag, updated.id)
    } else {
        // Create new event
        use crate::db::events::EventTiming;
        use televent_core::models::Timezone;

        let timing = if is_all_day {
            EventTiming::AllDay {
                date: start.date_naive(),
                end_date: end.date_naive(),
            }
        } else {
            EventTiming::Timed { start, end }
        };

        // Parse timezone (default to UTC if invalid)
        let tz = Timezone::parse(&timezone).unwrap_or_default();

        let created = db::events::create_event(
            &mut *tx,
            user.id,
            uid.to_string(),
            summary,
            description,
            location,
            timing,
            tz,
            rrule,
        )
        .await?;
        (StatusCode::CREATED, created.etag, created.id)
    };

    // Process Attendees in bulk
    let mut internal_attendees = std::collections::HashMap::new();
    for property in &event.properties {
        if property.name == "ATTENDEE"
            && let Some(value) = &property.value
        {
            // value is usually "mailto:email@example.com"
            let email = value.trim_start_matches("mailto:");
            if let Some(internal_user_id) = attendee::parse_internal_email(email) {
                // Skip self
                if internal_user_id != user.id {
                    // Collect unique attendees by email
                    internal_attendees.insert(
                        email.to_string(),
                        (internal_user_id, ParticipationStatus::NeedsAction),
                    );
                }
            }
        }
    }

    if !internal_attendees.is_empty() {
        let attendees_to_upsert = internal_attendees
            .into_iter()
            .map(|(email, (uid, status))| (uid, email, status))
            .collect();

        let upsert_results = db::events::upsert_event_attendees_bulk(
            &mut *tx,
            event_id,
            attendees_to_upsert,
        )
        .await?;

        let mut notifications = Vec::new();
        for res in upsert_results {
            if res.is_new {
                notifications.push((
                    "invite_notification",
                    serde_json::json!({
                        "event_id": event_id,
                        "target_user_id": res.user_id
                    }),
                ));
            }
        }

        if !notifications.is_empty() {
            db::events::create_outbox_messages_bulk(&mut *tx, notifications).await?;
        }
    }

    // Increment sync token
    let _new_sync_token = db::users::increment_sync_token_tx(&mut tx, user.id).await?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((status_code, [(header::ETAG, format!("\"{}\"", etag))], "").into_response())
}

/// CalDAV REPORT handler
///
/// Handles calendar-query and sync-collection reports (RFC 4791, RFC 6578)
async fn caldav_report(
    State(pool): State<PgPool>,
    Path(user_identifier): Path<String>,
    auth_user_id: UserId,
    body: Body,
) -> Result<Response, ApiError> {
    let user = resolve_user(&pool, &user_identifier).await?;

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
            // Query events with optional time range
            let events = db::events::list_events(&pool, user.id, start, end, None, None).await?;

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
            // Parse sync token to get last known state
            let last_sync_token = sync_token
                .as_ref()
                .and_then(|s: &String| s.rsplit('/').next())
                .and_then(|s: &str| s.parse::<i64>().ok())
                .unwrap_or(0);

            tracing::info!(
                "SyncCollection: sync_token={:?}, parsed={}",
                sync_token,
                last_sync_token
            );

            // Get events modified since last sync
            // For now, if sync_token is 0 or missing, return all events
            let events = if last_sync_token == 0 {
                db::events::list_events(&pool, user.id, None, None, None, None).await?
            } else {
                db::events::list_events_since_sync(&pool, user.id, last_sync_token).await?
            };

            tracing::info!(
                "SyncCollection: returning {} events, user sync_token={}",
                events.len(),
                user.sync_token
            );

            // Fetch deleted events
            let response_xml =
                caldav_xml::generate_sync_collection_response(&user_identifier, &user, &events)?;

            // Log the actual XML response for debugging
            if !events.is_empty() {
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

            // Fetch events by UIDs in batch
            let uid_strs: Vec<&str> = requested_uids.iter().map(|s| s.as_ref()).collect();
            let fetched_events = db::events::get_events_by_uids(&pool, user.id, &uid_strs).await?;

            let response_xml =
                caldav_xml::generate_calendar_multiget_response(&user_identifier, &fetched_events)?;

            tracing::info!(
                "CalendarMultiget: returning {} events",
                fetched_events.len()
            );

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
    State(pool): State<PgPool>,
    Path((user_identifier, event_uid)): Path<(String, String)>,
    auth_user_id: UserId,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let user = resolve_user(&pool, &user_identifier).await?;

    if user.id != auth_user_id {
        return Err(ApiError::Forbidden);
    }

    tracing::debug!("DELETE event {} for user {}", event_uid, user.id);

    // Check If-Match header for ETag (optional but recommended)
    if let Some(if_match) = headers.get(header::IF_MATCH) {
        let requested_etag = if_match
            .to_str()
            .map_err(|_| ApiError::BadRequest("Invalid If-Match header".to_string()))?;

        // Get current event to check ETag
        if let Some(event) = db::events::get_event_by_uid(&pool, user.id, &event_uid).await? {
            let current_etag = format!("\"{}\"", event.etag);
            if requested_etag != current_etag && requested_etag != "*" {
                return Err(ApiError::Conflict(format!(
                    "ETag mismatch: {} != {}",
                    requested_etag, current_etag
                )));
            }
        }
    }

    // Perform deletion and sync token increment in a transaction
    // This ensures the trigger in database captures the NEW sync token
    let mut tx = pool.begin().await?;

    // Increment sync token first so the deletion picks up the new token
    let _new_sync_token = db::users::increment_sync_token_tx(&mut tx, user.id).await?;

    // Delete event
    let deleted = db::events::delete_event_by_uid_tx(&mut tx, user.id, &event_uid).await?;

    if !deleted {
        return Err(ApiError::NotFound(format!(
            "Event not found: {}",
            event_uid
        )));
    }

    tx.commit().await?;

    Ok((StatusCode::NO_CONTENT, "").into_response())
}

/// Resolve user identifier (numeric ID or username) to User
///
/// The identifier can be:
/// - A numeric Telegram ID (e.g., "123456789")
/// - A Telegram username (e.g., "myusername")
async fn resolve_user(pool: &PgPool, identifier: &str) -> Result<User, ApiError> {
    // Try as numeric ID first
    if let Ok(telegram_id) = identifier.parse::<i64>() {
        let user = db::users::get_user_by_id(pool, UserId::new(telegram_id))
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("User not found: {identifier}")))?;
        return Ok(user);
    }

    // Try as username
    let user = db::users::get_user_by_username(pool, identifier)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {identifier}")))?;

    Ok(user)
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
    PgPool: FromRef<S>,
{
    Router::new()
        // Calendar collection endpoints
        .route("/{user_identifier}/", any(caldav_handler))
        // Event resource endpoints
        .route("/{user_identifier}/{*event_uid}", any(event_handler))
}

/// Main CalDAV collection handler
async fn caldav_handler(
    State(pool): State<PgPool>,
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
                State(pool),
                Path(user_identifier),
                auth_user_id,
                headers,
                body,
            )
            .await
        }
        "REPORT" => caldav_report(State(pool), Path(user_identifier), auth_user_id, body).await,
        _ => Err(ApiError::BadRequest(format!(
            "Method {} not supported for calendar collection",
            method
        ))),
    }
}

/// Event resource handler
async fn event_handler(
    State(pool): State<PgPool>,
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
                State(pool),
                Path((user_identifier, event_uid)),
                auth_user_id,
            )
            .await
        }
        Method::PUT => {
            caldav_put_event(
                State(pool),
                Path((user_identifier, event_uid)),
                auth_user_id,
                headers,
                body,
            )
            .await
        }
        Method::DELETE => {
            caldav_delete_event(
                State(pool),
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
        match uids[4] {
            Cow::Borrowed(_) => panic!("uid 5 should be owned"),
            Cow::Owned(_) => {} // OK
        }
    }
}
