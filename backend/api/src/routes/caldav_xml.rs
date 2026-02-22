//! CalDAV XML generation utilities
//!
//! Handles XML generation for CalDAV protocol responses

use chrono::{DateTime, Utc};
use quick_xml::Reader;
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::io::Cursor;
use televent_core::models::{CALENDAR_NAME, Event as CalEvent, User};
// use uuid::Uuid;

use super::ical;
use crate::error::ApiError;

/// Maximum number of hrefs allowed in a calendar-multiget report
const MAX_MULTIGET_HREFS: usize = 200;

/// Parsed REPORT request data
#[derive(Debug)]
pub enum ReportType {
    /// calendar-query: Query events with optional time-range filter
    CalendarQuery {
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    },
    /// sync-collection: Get changes since sync-token
    SyncCollection { sync_token: Option<String> },
    /// calendar-multiget: Fetch multiple specific calendar resources
    CalendarMultiget { hrefs: Vec<String> },
}

/// Parse CalDAV REPORT request XML
pub fn parse_report_request(xml_body: &str) -> Result<ReportType, ApiError> {
    let mut reader = Reader::from_str(xml_body);
    // Note: trim_text() removed in quick-xml 0.39, text is trimmed by default

    let mut in_calendar_query = false;
    let mut in_sync_collection = false;
    let mut in_calendar_multiget = false;
    let mut in_sync_token = false;
    let mut in_href = false;
    let mut _in_time_range = false;
    let mut sync_token: Option<String> = None;
    let mut time_range_start: Option<DateTime<Utc>> = None;
    let mut time_range_end: Option<DateTime<Utc>> = None;
    let mut hrefs: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");

                match name {
                    "calendar-query" => in_calendar_query = true,
                    "sync-collection" => in_sync_collection = true,
                    "calendar-multiget" => in_calendar_multiget = true,
                    "href" => in_href = true,
                    "time-range" => {
                        _in_time_range = true;
                        // Parse start/end attributes
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let value = std::str::from_utf8(&attr.value).unwrap_or("");
                            match key {
                                "start" => {
                                    time_range_start = parse_caldav_datetime(value);
                                }
                                "end" => {
                                    time_range_end = parse_caldav_datetime(value);
                                }
                                _ => {}
                            }
                        }
                    }
                    "sync-token" => {
                        in_sync_token = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                // Handle self-closing elements - they don't have text content
                let local_name = e.local_name();
                let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");

                match name {
                    "calendar-query" => in_calendar_query = true,
                    "sync-collection" => in_sync_collection = true,
                    "calendar-multiget" => in_calendar_multiget = true,
                    "time-range" => {
                        // Parse start/end attributes
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let value = std::str::from_utf8(&attr.value).unwrap_or("");
                            match key {
                                "start" => {
                                    time_range_start = parse_caldav_datetime(value);
                                }
                                "end" => {
                                    time_range_end = parse_caldav_datetime(value);
                                }
                                _ => {}
                            }
                        }
                    }
                    // Empty sync-token means initial sync (no previous token)
                    "sync-token" => {}
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = std::str::from_utf8(e.as_ref()).unwrap_or("");
                if in_sync_token && !text.is_empty() {
                    sync_token = Some(text.to_string());
                } else if in_href && !text.is_empty() {
                    if hrefs.len() >= MAX_MULTIGET_HREFS {
                        return Err(ApiError::BadRequest(format!(
                            "Too many hrefs in calendar-multiget (max {})",
                            MAX_MULTIGET_HREFS
                        )));
                    }
                    hrefs.push(text.to_string());
                }
            }
            Ok(Event::End(e)) => {
                let local_name = e.local_name();
                let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                match name {
                    "time-range" => _in_time_range = false,
                    "sync-token" => in_sync_token = false,
                    "href" => in_href = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Ok(Event::DocType(_)) => {
                return Err(ApiError::BadRequest("DTD not allowed".to_string()));
            }
            Err(e) => {
                return Err(ApiError::BadRequest(format!("XML parse error: {}", e)));
            }
            _ => {}
        }
    }

    if in_calendar_query {
        Ok(ReportType::CalendarQuery {
            start: time_range_start,
            end: time_range_end,
        })
    } else if in_sync_collection {
        Ok(ReportType::SyncCollection { sync_token })
    } else if in_calendar_multiget {
        Ok(ReportType::CalendarMultiget { hrefs })
    } else {
        Err(ApiError::BadRequest(
            "Unknown REPORT type: expected calendar-query, sync-collection, or calendar-multiget"
                .to_string(),
        ))
    }
}

/// Parse CalDAV datetime format (ISO 8601 basic format)
fn parse_caldav_datetime(s: &str) -> Option<DateTime<Utc>> {
    // CalDAV uses format like: 20240101T000000Z
    chrono::NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%SZ")
        .ok()
        .map(|dt| dt.and_utc())
}

/// Generate CalDAV multistatus response for REPORT calendar-query
pub fn generate_calendar_query_response(
    user_identifier: &str,
    events: &[CalEvent],
) -> Result<String, ApiError> {
    // Pre-allocate buffer: ~512 bytes per event to minimize reallocations
    let capacity = events.len() * 512 + 1024;
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::with_capacity(capacity)), b' ', 2);

    // XML declaration
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <multistatus>
    let mut multistatus = BytesStart::new("d:multistatus");
    multistatus.push_attribute(("xmlns:d", "DAV:"));
    multistatus.push_attribute(("xmlns:cal", "urn:ietf:params:xml:ns:caldav"));
    multistatus.push_attribute(("xmlns:cs", "http://calendarserver.org/ns/"));
    writer
        .write_event(Event::Start(multistatus))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // Reusable buffer for iCalendar data
    let mut ical_buf = String::with_capacity(1024);

    // Write response for each event with calendar-data
    for event in events {
        ical_buf.clear();
        ical::event_to_ical_into(event, &[], &mut ical_buf)?;
        write_event_with_data(&mut writer, user_identifier, event, &ical_buf)?;
    }

    // </multistatus>
    writer
        .write_event(Event::End(BytesEnd::new("d:multistatus")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| ApiError::Internal(format!("UTF-8 error: {}", e)))
}

/// Generate CalDAV multistatus response for REPORT sync-collection
pub fn generate_sync_collection_response(
    user_identifier: &str,
    events: &[CalEvent],
    sync_token: &str,
) -> Result<String, ApiError> {
    // Pre-allocate buffer: ~512 bytes per event to minimize reallocations
    let capacity = events.len() * 512 + 1024;
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::with_capacity(capacity)), b' ', 2);

    // XML declaration
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <multistatus>
    let mut multistatus = BytesStart::new("d:multistatus");
    multistatus.push_attribute(("xmlns:d", "DAV:"));
    multistatus.push_attribute(("xmlns:cal", "urn:ietf:params:xml:ns:caldav"));
    multistatus.push_attribute(("xmlns:cs", "http://calendarserver.org/ns/"));
    writer
        .write_event(Event::Start(multistatus))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // Reusable buffer for iCalendar data
    let mut ical_buf = String::with_capacity(1024);

    // Write response for changed/new events with calendar-data
    for event in events {
        ical_buf.clear();
        ical::event_to_ical_into(event, &[], &mut ical_buf)?;
        write_event_with_data(&mut writer, user_identifier, event, &ical_buf)?;
    }

    // <sync-token> - use write! to avoid allocation
    {
        use std::fmt::Write;
        let mut sync_token_buf = String::with_capacity(48);
        write!(
            sync_token_buf,
            "http://televent.app/sync/{}",
            sync_token
        )
        .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
        writer
            .write_event(Event::Start(BytesStart::new("d:sync-token")))
            .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
        writer
            .write_event(Event::Text(BytesText::new(&sync_token_buf)))
            .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
        writer
            .write_event(Event::End(BytesEnd::new("d:sync-token")))
            .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    }

    // </multistatus>
    writer
        .write_event(Event::End(BytesEnd::new("d:multistatus")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| ApiError::Internal(format!("UTF-8 error: {}", e)))
}

/// Generate CalDAV multistatus response for REPORT calendar-multiget
pub fn generate_calendar_multiget_response(
    user_identifier: &str,
    events: &[CalEvent],
) -> Result<String, ApiError> {
    // Pre-allocate buffer: ~512 bytes per event to minimize reallocations
    let capacity = events.len() * 512 + 1024;
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::with_capacity(capacity)), b' ', 2);

    // XML declaration
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <multistatus>
    let mut multistatus = BytesStart::new("d:multistatus");
    multistatus.push_attribute(("xmlns:d", "DAV:"));
    multistatus.push_attribute(("xmlns:cal", "urn:ietf:params:xml:ns:caldav"));
    multistatus.push_attribute(("xmlns:cs", "http://calendarserver.org/ns/"));
    writer
        .write_event(Event::Start(multistatus))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // Reusable buffer for iCalendar data
    let mut ical_buf = String::with_capacity(1024);

    // Write response for each event with calendar-data
    for event in events {
        ical_buf.clear();
        match ical::event_to_ical_into(event, &[], &mut ical_buf) {
            Ok(()) => {
                write_event_with_data(&mut writer, user_identifier, event, &ical_buf)?;
            }
            Err(e) => {
                tracing::warn!("Failed to generate iCalendar for {}: {:?}", event.uid, e);
                continue;
            }
        }
    }

    // </multistatus>
    writer
        .write_event(Event::End(BytesEnd::new("d:multistatus")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| ApiError::Internal(format!("UTF-8 error: {}", e)))
}

/// Write event response with calendar-data (for REPORT)
///
/// Uses a reusable buffer to avoid repeated string allocations in hot loops.
fn write_event_with_data(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    user_identifier: &str,
    event: &CalEvent,
    ical_data: &str,
) -> Result<(), ApiError> {
    use std::fmt::Write;

    // Reusable buffer to avoid allocations per event (capacity for typical href)
    let mut buf = String::with_capacity(128);

    // <response>
    write_start_tag(writer, "d:response")?;

    // <href> - reuse buffer instead of format!
    buf.clear();
    write!(buf, "/caldav/{}/{}.ics", user_identifier, event.uid)
        .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:href", &buf)?;

    // <propstat>
    write_start_tag(writer, "d:propstat")?;

    // <prop>
    write_start_tag(writer, "d:prop")?;

    // <getetag> - reuse buffer
    buf.clear();
    write!(buf, "\"{}\"", event.etag)
        .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:getetag", &buf)?;

    // <getcontenttype>
    write_string_tag(writer, "d:getcontenttype", "text/calendar; charset=utf-8")?;

    // <getlastmodified> (RFC 2616 HTTP-date format) - reuse buffer
    buf.clear();
    write!(
        buf,
        "{}",
        event.updated_at.format("%a, %d %b %Y %H:%M:%S GMT")
    )
    .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:getlastmodified", &buf)?;

    // <calendar-data>
    write_string_tag(writer, "cal:calendar-data", ical_data)?;

    // </prop>
    write_end_tag(writer, "d:prop")?;

    // <status>HTTP/1.1 200 OK</status>
    write_string_tag(writer, "d:status", "HTTP/1.1 200 OK")?;

    // </propstat>
    write_end_tag(writer, "d:propstat")?;

    // </response>
    write_end_tag(writer, "d:response")
}

/// Generate CalDAV multistatus response for PROPFIND
pub fn generate_propfind_multistatus(
    user_identifier: &str,
    user: &User,
    events: &[CalEvent],
    depth: &str,
) -> Result<String, ApiError> {
    // Pre-allocate buffer if we are returning events (Depth: 1)
    let capacity = if depth == "1" {
        events.len() * 512 + 2048
    } else {
        4096 // Enough for calendar properties
    };
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::with_capacity(capacity)), b' ', 2);

    // XML declaration
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <multistatus> root element
    let mut multistatus = BytesStart::new("d:multistatus");
    multistatus.push_attribute(("xmlns:d", "DAV:"));
    multistatus.push_attribute(("xmlns:cal", "urn:ietf:params:xml:ns:caldav"));
    multistatus.push_attribute(("xmlns:cs", "http://calendarserver.org/ns/"));
    writer
        .write_event(Event::Start(multistatus))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // Calendar collection response (user = calendar)
    write_calendar_response(&mut writer, user_identifier, user)?;

    // Event responses (only for Depth: 1)
    if depth == "1" {
        for event in events {
            write_event_response(&mut writer, user_identifier, event)?;
        }
    }

    // Close </multistatus>
    writer
        .write_event(Event::End(BytesEnd::new("d:multistatus")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| ApiError::Internal(format!("UTF-8 error: {}", e)))
}

/// Write calendar collection response (user = calendar)
///
/// Uses a reusable buffer to reduce allocations.
fn write_calendar_response(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    user_identifier: &str,
    user: &User,
) -> Result<(), ApiError> {
    use std::fmt::Write;

    // Reusable buffer for formatted strings
    let mut buf = String::with_capacity(64);

    // <response>
    write_start_tag(writer, "d:response")?;

    // <href>/caldav/{user_id}/</href> - reuse buffer
    buf.clear();
    write!(buf, "/caldav/{}/", user_identifier)
        .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:href", &buf)?;

    // <propstat>
    write_start_tag(writer, "d:propstat")?;

    // <prop>
    write_start_tag(writer, "d:prop")?;

    // <resourcetype><collection/><calendar/></resourcetype>
    write_start_tag(writer, "d:resourcetype")?;
    write_empty_tag(writer, "d:collection")?;
    write_empty_tag(writer, "cal:calendar")?;
    write_end_tag(writer, "d:resourcetype")?;

    // <displayname>
    write_string_tag(writer, "d:displayname", CALENDAR_NAME)?;

    // <getctag>
    write_string_tag(writer, "cal:getctag", &user.ctag)?;

    // <sync-token> (RFC 6578) - reuse buffer
    buf.clear();
    write!(buf, "http://televent.app/sync/{}", user.sync_token)
        .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:sync-token", &buf)?;

    // <calendar-home-set> - reuse the href we already formatted
    write_start_tag(writer, "cal:calendar-home-set")?;
    buf.clear();
    write!(buf, "/caldav/{}/", user_identifier)
        .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:href", &buf)?;
    write_end_tag(writer, "cal:calendar-home-set")?;

    // <current-user-principal> - href is same as calendar-home-set
    write_start_tag(writer, "d:current-user-principal")?;
    write_string_tag(writer, "d:href", &buf)?;
    write_end_tag(writer, "d:current-user-principal")?;

    // <owner> - href is same
    write_start_tag(writer, "d:owner")?;
    write_string_tag(writer, "d:href", &buf)?;
    write_end_tag(writer, "d:owner")?;

    // <supported-calendar-component-set>
    write_start_tag(writer, "cal:supported-calendar-component-set")?;
    // We can't use write_empty_tag directly as it needs an attribute
    // Custom logic for this one small part is fine, or extend helpers, but keeping it simple:
    let mut comp = BytesStart::new("cal:comp");
    comp.push_attribute(("name", "VEVENT"));
    writer
        .write_event(Event::Empty(comp))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    write_end_tag(writer, "cal:supported-calendar-component-set")?;

    // <supported-report-set>
    write_start_tag(writer, "d:supported-report-set")?;

    // report: calendar-query
    write_start_tag(writer, "d:supported-report")?;
    write_start_tag(writer, "d:report")?;
    write_empty_tag(writer, "cal:calendar-query")?;
    write_end_tag(writer, "d:report")?;
    write_end_tag(writer, "d:supported-report")?;

    // report: sync-collection
    write_start_tag(writer, "d:supported-report")?;
    write_start_tag(writer, "d:report")?;
    write_empty_tag(writer, "d:sync-collection")?;
    write_end_tag(writer, "d:report")?;
    write_end_tag(writer, "d:supported-report")?;

    write_end_tag(writer, "d:supported-report-set")?;

    // </prop>
    write_end_tag(writer, "d:prop")?;

    // <status>HTTP/1.1 200 OK</status>
    write_string_tag(writer, "d:status", "HTTP/1.1 200 OK")?;

    // </propstat>
    write_end_tag(writer, "d:propstat")?;

    // </response>
    write_end_tag(writer, "d:response")
}

/// Write event resource response
///
/// Uses a reusable buffer to avoid repeated string allocations in hot loops.
fn write_event_response(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    user_identifier: &str,
    event: &CalEvent,
) -> Result<(), ApiError> {
    use std::fmt::Write;

    // Reusable buffer to avoid allocations per event
    let mut buf = String::with_capacity(128);

    // <response>
    write_start_tag(writer, "d:response")?;

    // <href>/caldav/{user_id}/{uid}.ics</href> - reuse buffer
    buf.clear();
    write!(buf, "/caldav/{}/{}.ics", user_identifier, event.uid)
        .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:href", &buf)?;

    // <propstat>
    write_start_tag(writer, "d:propstat")?;

    // <prop>
    write_start_tag(writer, "d:prop")?;

    // <getetag> - reuse buffer
    buf.clear();
    write!(buf, "\"{}\"", event.etag)
        .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:getetag", &buf)?;

    // <getcontenttype>
    write_string_tag(writer, "d:getcontenttype", "text/calendar; charset=utf-8")?;

    // <getlastmodified> (RFC 2616 HTTP-date format) - reuse buffer
    buf.clear();
    write!(
        buf,
        "{}",
        event.updated_at.format("%a, %d %b %Y %H:%M:%S GMT")
    )
    .map_err(|e| ApiError::Internal(format!("Format error: {}", e)))?;
    write_string_tag(writer, "d:getlastmodified", &buf)?;

    // </prop>
    write_end_tag(writer, "d:prop")?;

    // <status>HTTP/1.1 200 OK</status>
    write_string_tag(writer, "d:status", "HTTP/1.1 200 OK")?;

    // </propstat>
    write_end_tag(writer, "d:propstat")?;

    // </response>
    write_end_tag(writer, "d:response")
}

/// Write a simple XML element with text content: <tag>content</tag>
fn write_string_tag<W: std::io::Write>(
    writer: &mut Writer<W>,
    tag: &str,
    text: &str,
) -> Result<(), ApiError> {
    writer
        .write_event(Event::Start(BytesStart::new(tag)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    Ok(())
}

/// Start an element: <tag>
fn write_start_tag<W: std::io::Write>(writer: &mut Writer<W>, tag: &str) -> Result<(), ApiError> {
    writer
        .write_event(Event::Start(BytesStart::new(tag)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))
}

/// End an element: </tag>
fn write_end_tag<W: std::io::Write>(writer: &mut Writer<W>, tag: &str) -> Result<(), ApiError> {
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))
}

/// Write an empty element: <tag/>
fn write_empty_tag<W: std::io::Write>(writer: &mut Writer<W>, tag: &str) -> Result<(), ApiError> {
    writer
        .write_event(Event::Empty(BytesStart::new(tag)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Utc};
    use televent_core::models::{EventStatus, Timezone, UserId};
    use uuid::Uuid;

    // Test helper: create a User fixture
    fn test_user() -> User {
        let now = Utc::now();
        User {
            id: UserId::new(123456789),
            telegram_username: Some("testuser".to_string()),
            timezone: Timezone::default(),
            sync_token: "1".to_string(),
            ctag: "123456".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    // Test helper: create an Event fixture
    fn test_event(uid: &str) -> CalEvent {
        let now = Utc::now();
        CalEvent {
            id: Uuid::new_v4(),
            user_id: UserId::new(123456789),
            uid: uid.to_string(),
            version: 1,
            etag: "abc123".to_string(),
            summary: "Test Event".to_string(),
            description: None,
            location: None,
            start: Some(now),
            end: Some(now),
            start_date: None,
            end_date: None,
            is_all_day: false,
            rrule: None,
            status: EventStatus::Confirmed,
            timezone: Timezone::default(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_generate_propfind_depth_0() {
        let user = test_user();

        let xml = generate_propfind_multistatus("testuser", &user, &[], "0").unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        assert!(xml.contains(CALENDAR_NAME));
        assert!(xml.contains("123456")); // ctag
        assert!(xml.contains("VEVENT"));
        // Should not contain any event hrefs for depth 0
        assert!(!xml.contains(".ics"));
    }

    #[test]
    fn test_generate_propfind_depth_1() {
        let user = test_user();
        let event = test_event("test-event-1");

        let xml = generate_propfind_multistatus("testuser", &user, &[event], "1").unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        assert!(xml.contains(CALENDAR_NAME));
        // Should contain event href for depth 1
        assert!(xml.contains("test-event-1.ics"));
        assert!(xml.contains("abc123")); // etag
        assert!(xml.contains("text/calendar"));
    }

    #[test]
    fn test_xml_structure_valid() {
        let mut user = test_user();
        user.sync_token = "0".to_string();
        user.ctag = "0".to_string();

        let xml = generate_propfind_multistatus("testuser", &user, &[], "0").unwrap();

        // Check XML declaration
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        // Check namespaces
        assert!(xml.contains("xmlns:d=\"DAV:\""));
        assert!(xml.contains("xmlns:cal=\"urn:ietf:params:xml:ns:caldav\""));
        // Check required properties
        assert!(xml.contains("<d:resourcetype>"));
        assert!(xml.contains("<d:collection/>"));
        assert!(xml.contains("<cal:calendar/>"));
        assert!(xml.contains("<d:displayname>"));
        assert!(xml.contains("<cal:getctag>"));
        assert!(xml.contains("<cal:supported-calendar-component-set>"));
        assert!(xml.contains("HTTP/1.1 200 OK"));
    }

    #[test]
    fn test_parse_report_calendar_query_basic() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
            <C:calendar-query xmlns:C="urn:ietf:params:xml:ns:caldav">
                <D:prop xmlns:D="DAV:">
                    <D:getetag/>
                    <C:calendar-data/>
                </D:prop>
            </C:calendar-query>"#;

        let result = parse_report_request(xml).unwrap();
        match result {
            ReportType::CalendarQuery { start, end } => {
                assert!(start.is_none());
                assert!(end.is_none());
            }
            _ => panic!("Expected CalendarQuery"),
        }
    }

    #[test]
    fn test_parse_report_calendar_query_with_time_range() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
            <C:calendar-query xmlns:C="urn:ietf:params:xml:ns:caldav">
                <C:filter>
                    <C:comp-filter name="VCALENDAR">
                        <C:comp-filter name="VEVENT">
                            <C:time-range start="20240101T000000Z" end="20240201T000000Z"/>
                        </C:comp-filter>
                    </C:comp-filter>
                </C:filter>
            </C:calendar-query>"#;

        let result = parse_report_request(xml).unwrap();
        match result {
            ReportType::CalendarQuery { start, end } => {
                assert!(start.is_some());
                assert!(end.is_some());
                let start = start.unwrap();
                let end = end.unwrap();
                assert_eq!(start.year(), 2024);
                assert_eq!(start.month(), 1);
                assert_eq!(start.day(), 1);
                assert_eq!(end.month(), 2);
            }
            _ => panic!("Expected CalendarQuery"),
        }
    }

    #[test]
    fn test_parse_report_sync_collection_initial() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
            <D:sync-collection xmlns:D="DAV:">
                <D:sync-token/>
                <D:sync-level>1</D:sync-level>
                <D:prop>
                    <D:getetag/>
                </D:prop>
            </D:sync-collection>"#;

        let result = parse_report_request(xml).unwrap();
        match result {
            ReportType::SyncCollection { sync_token } => {
                assert!(sync_token.is_none());
            }
            _ => panic!("Expected SyncCollection"),
        }
    }

    #[test]
    fn test_parse_report_sync_collection_with_token() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
            <D:sync-collection xmlns:D="DAV:">
                <D:sync-token>http://televent.app/sync/42</D:sync-token>
                <D:sync-level>1</D:sync-level>
            </D:sync-collection>"#;

        let result = parse_report_request(xml).unwrap();
        match result {
            ReportType::SyncCollection { sync_token } => {
                assert!(sync_token.is_some());
                assert!(sync_token.unwrap().contains("42"));
            }
            _ => panic!("Expected SyncCollection"),
        }
    }

    #[test]
    fn test_parse_report_unknown_type() {
        let xml = r#"<?xml version="1.0"?>
            <D:unknown-report xmlns:D="DAV:"/>"#;

        let result = parse_report_request(xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_calendar_query_response() {
        let event = test_event("event-123");
        let xml = generate_calendar_query_response("testuser", &[event]).unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("event-123.ics"));
        assert!(xml.contains("abc123"));
        assert!(xml.contains("cal:calendar-data"));
        assert!(xml.contains("BEGIN:VCALENDAR"));
        assert!(xml.contains("HTTP/1.1 200 OK"));
    }

    #[test]
    fn test_generate_sync_collection_response_with_changes() {
        let mut user = test_user();
        user.sync_token = "55".to_string();
        user.ctag = "ctag-123".to_string();

        let mut event = test_event("changed-event");
        event.version = 2;
        event.etag = "new-etag".to_string();
        event.summary = "Updated Event".to_string();

        let xml = generate_sync_collection_response("testuser", &[event], "55").unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        // Changed event should have 200 status
        assert!(xml.contains("changed-event.ics"));
        assert!(xml.contains("new-etag"));
        // New sync token
        assert!(xml.contains("<d:sync-token>"));
        assert!(xml.contains("/sync/55"));
    }

    #[test]
    fn test_generate_sync_collection_response_empty() {
        let mut _user = test_user();
        _user.sync_token = "100".to_string();

        let xml = generate_sync_collection_response("testuser", &[], "100").unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("<d:sync-token>"));
        assert!(xml.contains("/sync/100"));
        // Should not contain any event responses
        assert!(!xml.contains(".ics"));
    }

    #[test]
    #[ignore] // benchmark
    fn test_benchmark_generate_calendar_query_response() {
        let count = 2000;
        let mut events = Vec::with_capacity(count);

        for i in 0..count {
            let uid = format!("event-{}", i);
            let mut event = test_event(&uid);
            event.etag = format!("etag-{}", i);
            event.summary = format!("Event {}", i);

            events.push(event);
        }

        let start = std::time::Instant::now();
        let _ = generate_calendar_query_response("testuser", &events).unwrap();
        let duration = start.elapsed();

        println!(
            "Benchmark generate_calendar_query_response (N={}): {:?}",
            count, duration
        );
    }

    #[test]
    fn test_parse_report_calendar_multiget_limit_exceeded() {
        use std::fmt::Write;
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="utf-8"?>
            <C:calendar-multiget xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
                <D:prop>
                    <D:getetag/>
                    <C:calendar-data/>
                </D:prop>"#,
        );

        // Add MAX_MULTIGET_HREFS + 1 hrefs
        for i in 0..MAX_MULTIGET_HREFS + 1 {
            write!(xml, "<D:href>/caldav/user/event-{}.ics</D:href>", i).unwrap();
        }

        xml.push_str("</C:calendar-multiget>");

        let result = parse_report_request(&xml);
        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::BadRequest(msg) => {
                assert!(msg.contains("Too many hrefs"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[test]
    fn test_parse_report_calendar_multiget_limit_ok() {
        use std::fmt::Write;
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="utf-8"?>
            <C:calendar-multiget xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
                <D:prop>
                    <D:getetag/>
                    <C:calendar-data/>
                </D:prop>"#,
        );

        // Add exactly MAX_MULTIGET_HREFS hrefs
        for i in 0..MAX_MULTIGET_HREFS {
            write!(xml, "<D:href>/caldav/user/event-{}.ics</D:href>", i).unwrap();
        }

        xml.push_str("</C:calendar-multiget>");

        let result = parse_report_request(&xml);
        assert!(result.is_ok());
        match result.unwrap() {
            ReportType::CalendarMultiget { hrefs } => {
                assert_eq!(hrefs.len(), MAX_MULTIGET_HREFS);
            }
            _ => panic!("Expected CalendarMultiget"),
        }
    }
}
