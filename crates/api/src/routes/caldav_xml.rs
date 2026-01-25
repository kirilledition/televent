//! CalDAV XML generation utilities
//!
//! Handles XML generation for CalDAV protocol responses

use chrono::{DateTime, Utc};
use quick_xml::Reader;
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::collections::HashMap;
use std::io::Cursor;
use televent_core::models::{Calendar, Event as CalEvent};
// use uuid::Uuid;

use crate::error::ApiError;

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
    ical_data: &[(String, String)], // (uid, ical_string)
) -> Result<String, ApiError> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

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

    // Create a lookup map for ical data
    let ical_map = build_uid_map(ical_data);

    // Write response for each event with calendar-data
    for event in events {
        let ical = ical_map.get(event.uid.as_str()).copied().unwrap_or("");
        write_event_with_data(&mut writer, user_identifier, event, ical)?;
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
    calendar: &Calendar,
    events: &[CalEvent],
    ical_data: &[(String, String)], // (uid, ical_string)
    deleted_uids: &[String],
) -> Result<String, ApiError> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

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

    // Create a lookup map for ical data
    let ical_map = build_uid_map(ical_data);

    // Write response for changed/new events with calendar-data
    for event in events {
        let ical = ical_map.get(event.uid.as_str()).copied().unwrap_or("");
        write_event_with_data(&mut writer, user_identifier, event, ical)?;
    }

    // Write 404 response for deleted events
    for uid in deleted_uids {
        write_deleted_event_response(&mut writer, user_identifier, uid)?;
    }

    // <sync-token>
    writer
        .write_event(Event::Start(BytesStart::new("d:sync-token")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(&format!(
            "http://televent.app/sync/{}",
            calendar.sync_token
        ))))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:sync-token")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

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
    ical_data: &[(String, String)], // (uid, ical_string)
) -> Result<String, ApiError> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

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

    // Create a lookup map for ical data
    let ical_map = build_uid_map(ical_data);

    // Write response for each event with calendar-data
    for event in events {
        let ical = ical_map.get(event.uid.as_str()).copied().unwrap_or("");
        write_event_with_data(&mut writer, user_identifier, event, ical)?;
    }

    // </multistatus>
    writer
        .write_event(Event::End(BytesEnd::new("d:multistatus")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| ApiError::Internal(format!("UTF-8 error: {}", e)))
}

/// Helper to build O(1) lookup map for ical data
fn build_uid_map(ical_data: &[(String, String)]) -> HashMap<&str, &str> {
    ical_data
        .iter()
        .map(|(uid, data)| (uid.as_str(), data.as_str()))
        .collect()
}

/// Write event response with calendar-data (for REPORT)
fn write_event_with_data(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    user_identifier: &str,
    event: &CalEvent,
    ical_data: &str,
) -> Result<(), ApiError> {
    // <response>
    write_start_tag(writer, "d:response")?;

    // <href>
    write_string_tag(
        writer,
        "d:href",
        &format!("/caldav/{}/{}.ics", user_identifier, event.uid),
    )?;

    // <propstat>
    write_start_tag(writer, "d:propstat")?;

    // <prop>
    write_start_tag(writer, "d:prop")?;

    // <getetag>
    write_string_tag(writer, "d:getetag", &format!("\"{}\"", event.etag))?;

    // <getcontenttype>
    write_string_tag(writer, "d:getcontenttype", "text/calendar; charset=utf-8")?;

    // <getlastmodified> (RFC 2616 HTTP-date format)
    let http_date = event
        .updated_at
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();
    write_string_tag(writer, "d:getlastmodified", &http_date)?;

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

/// Write 404 response for deleted event (sync-collection)
fn write_deleted_event_response(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    user_identifier: &str,
    uid: &str,
) -> Result<(), ApiError> {
    // <response>
    write_start_tag(writer, "d:response")?;

    // <href>
    write_string_tag(
        writer,
        "d:href",
        &format!("/caldav/{}/{}.ics", user_identifier, uid),
    )?;

    // <status>HTTP/1.1 404 Not Found</status>
    write_string_tag(writer, "d:status", "HTTP/1.1 404 Not Found")?;

    // </response>
    write_end_tag(writer, "d:response")
}

/// Generate CalDAV multistatus response for PROPFIND
pub fn generate_propfind_multistatus(
    user_identifier: &str,
    calendar: &Calendar,
    events: &[CalEvent],
    depth: &str,
) -> Result<String, ApiError> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

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

    // Calendar collection response
    write_calendar_response(&mut writer, user_identifier, calendar)?;

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

/// Write calendar collection response
fn write_calendar_response(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    user_identifier: &str,
    calendar: &Calendar,
) -> Result<(), ApiError> {
    // <response>
    write_start_tag(writer, "d:response")?;

    // <href>/caldav/{user_id}/</href>
    write_string_tag(writer, "d:href", &format!("/caldav/{}/", user_identifier))?;

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
    write_string_tag(writer, "d:displayname", &calendar.name)?;

    // <getctag>
    write_string_tag(writer, "cal:getctag", &calendar.ctag)?;

    // <sync-token> (RFC 6578)
    write_string_tag(
        writer,
        "d:sync-token",
        &format!("http://televent.app/sync/{}", calendar.sync_token),
    )?;

    // <calendar-home-set>
    write_start_tag(writer, "cal:calendar-home-set")?;
    write_string_tag(writer, "d:href", &format!("/caldav/{}/", user_identifier))?;
    write_end_tag(writer, "cal:calendar-home-set")?;

    // <current-user-principal>
    write_start_tag(writer, "d:current-user-principal")?;
    // Simplified: principal is the same as calendar home
    write_string_tag(writer, "d:href", &format!("/caldav/{}/", user_identifier))?;
    write_end_tag(writer, "d:current-user-principal")?;

    // <owner>
    write_start_tag(writer, "d:owner")?;
    write_string_tag(writer, "d:href", &format!("/caldav/{}/", user_identifier))?;
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
fn write_event_response(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    user_identifier: &str,
    event: &CalEvent,
) -> Result<(), ApiError> {
    // <response>
    write_start_tag(writer, "d:response")?;

    // <href>/caldav/{user_id}/{uid}.ics</href>
    write_string_tag(
        writer,
        "d:href",
        &format!("/caldav/{}/{}.ics", user_identifier, event.uid),
    )?;

    // <propstat>
    write_start_tag(writer, "d:propstat")?;

    // <prop>
    write_start_tag(writer, "d:prop")?;

    // <getetag>
    write_string_tag(writer, "d:getetag", &format!("\"{}\"", event.etag))?;

    // <getcontenttype>
    write_string_tag(writer, "d:getcontenttype", "text/calendar; charset=utf-8")?;

    // <getlastmodified> (RFC 2616 HTTP-date format)
    let http_date = event
        .updated_at
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();
    write_string_tag(writer, "d:getlastmodified", &http_date)?;

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
    use televent_core::models::EventStatus;
    use televent_core::timezone::Timezone;
    use uuid::Uuid;

    #[test]
    fn test_generate_propfind_depth_0() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let calendar = Calendar {
            id: Uuid::new_v4(),
            user_id,
            name: "Test Calendar".to_string(),
            color: "#ff0000".to_string(),
            sync_token: "1".to_string(),
            ctag: "123456".to_string(),
            created_at: now,
            updated_at: now,
        };

        let xml = generate_propfind_multistatus("testuser", &calendar, &[], "0").unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("Test Calendar"));
        assert!(xml.contains("123456")); // ctag
        assert!(xml.contains("VEVENT"));
        // Should not contain any event hrefs for depth 0
        assert!(!xml.contains(".ics"));
    }

    #[test]
    fn test_generate_propfind_depth_1() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let calendar = Calendar {
            id: Uuid::new_v4(),
            user_id,
            name: "Test Calendar".to_string(),
            color: "#ff0000".to_string(),
            sync_token: "1".to_string(),
            ctag: "123456".to_string(),
            created_at: now,
            updated_at: now,
        };

        let now = Utc::now();
        let event = CalEvent {
            id: Uuid::new_v4(),
            calendar_id: calendar.id,
            uid: "test-event-1".to_string(),
            version: 1,
            etag: "abc123".to_string(),
            summary: "Test Event".to_string(),
            description: None,
            location: None,
            start: now,
            end: now,
            is_all_day: false,
            rrule: None,
            status: EventStatus::Confirmed,
            timezone: Timezone::new("UTC").unwrap(),
            created_at: now,
            updated_at: now,
        };

        let xml = generate_propfind_multistatus("testuser", &calendar, &[event], "1").unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("Test Calendar"));
        // Should contain event href for depth 1
        assert!(xml.contains("test-event-1.ics"));
        assert!(xml.contains("abc123")); // etag
        assert!(xml.contains("text/calendar"));
    }

    #[test]
    fn test_xml_structure_valid() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let calendar = Calendar {
            id: Uuid::new_v4(),
            user_id,
            name: "Test".to_string(),
            color: "#000000".to_string(),
            sync_token: "0".to_string(),
            ctag: "0".to_string(),
            created_at: now,
            updated_at: now,
        };

        let xml = generate_propfind_multistatus("testuser", &calendar, &[], "0").unwrap();

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
        let _user_id = Uuid::new_v4();
        let now = Utc::now();
        let calendar_id = Uuid::new_v4();

        let event = CalEvent {
            id: Uuid::new_v4(),
            calendar_id,
            uid: "event-123".to_string(),
            version: 1,
            etag: "etag-abc".to_string(),
            summary: "Test Event".to_string(),
            description: Some("Description".to_string()),
            location: None,
            start: now,
            end: now,
            is_all_day: false,
            rrule: None,
            status: EventStatus::Confirmed,
            timezone: Timezone::new("UTC").unwrap(),
            created_at: now,
            updated_at: now,
        };

        let ical_data = vec![(
            "event-123".to_string(),
            "BEGIN:VCALENDAR...END:VCALENDAR".to_string(),
        )];
        let xml = generate_calendar_query_response("testuser", &[event], &ical_data).unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("event-123.ics"));
        assert!(xml.contains("etag-abc"));
        assert!(xml.contains("cal:calendar-data"));
        assert!(xml.contains("BEGIN:VCALENDAR"));
        assert!(xml.contains("HTTP/1.1 200 OK"));
    }

    #[test]
    fn test_generate_sync_collection_response_with_changes() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let calendar_id = Uuid::new_v4();

        let calendar = Calendar {
            id: calendar_id,
            user_id,
            name: "Test".to_string(),
            color: "#000000".to_string(),
            sync_token: "55".to_string(),
            ctag: "ctag-123".to_string(),
            created_at: now,
            updated_at: now,
        };

        let event = CalEvent {
            id: Uuid::new_v4(),
            calendar_id,
            uid: "changed-event".to_string(),
            version: 2,
            etag: "new-etag".to_string(),
            summary: "Updated Event".to_string(),
            description: None,
            location: None,
            start: now,
            end: now,
            is_all_day: false,
            rrule: None,
            status: EventStatus::Confirmed,
            timezone: Timezone::new("UTC").unwrap(),
            created_at: now,
            updated_at: now,
        };

        let deleted_uids = vec!["deleted-event".to_string()];
        let xml =
            generate_sync_collection_response("testuser", &calendar, &[event], &[], &deleted_uids)
                .unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("multistatus"));
        // Changed event should have 200 status
        assert!(xml.contains("changed-event.ics"));
        assert!(xml.contains("new-etag"));
        // Deleted event should have 404 status
        assert!(xml.contains("deleted-event.ics"));
        assert!(xml.contains("HTTP/1.1 404 Not Found"));
        // New sync token
        assert!(xml.contains("<d:sync-token>"));
        assert!(xml.contains("/sync/55"));
    }

    #[test]
    fn test_generate_sync_collection_response_empty() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let calendar = Calendar {
            id: Uuid::new_v4(),
            user_id,
            name: "Test".to_string(),
            color: "#000000".to_string(),
            sync_token: "100".to_string(),
            ctag: "ctag".to_string(),
            created_at: now,
            updated_at: now,
        };

        let xml = generate_sync_collection_response("testuser", &calendar, &[], &[], &[]).unwrap();

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
        let _user_id = Uuid::new_v4();
        let now = Utc::now();
        let calendar_id = Uuid::new_v4();

        let count = 2000;
        let mut events = Vec::with_capacity(count);
        let mut ical_data = Vec::with_capacity(count);

        for i in 0..count {
            let uid = format!("event-{}", i);
            let event = CalEvent {
                id: Uuid::new_v4(),
                calendar_id,
                uid: uid.clone(),
                version: 1,
                etag: format!("etag-{}", i),
                summary: format!("Event {}", i),
                description: None,
                location: None,
                start: now,
                end: now,
                is_all_day: false,
                rrule: None,
                status: EventStatus::Confirmed,
                timezone: Timezone::new("UTC").unwrap(),
                created_at: now,
                updated_at: now,
            };
            events.push(event);
            ical_data.push((uid, "BEGIN:VCALENDAR...END:VCALENDAR".to_string()));
        }

        let start = std::time::Instant::now();
        let _ = generate_calendar_query_response("testuser", &events, &ical_data).unwrap();
        let duration = start.elapsed();

        println!(
            "Benchmark generate_calendar_query_response (N={}): {:?}",
            count, duration
        );
    }
}
