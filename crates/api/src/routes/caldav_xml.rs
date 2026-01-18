//! CalDAV XML generation utilities
//!
//! Handles XML generation for CalDAV protocol responses

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;
use televent_core::models::{Calendar, Event as CalEvent};
use uuid::Uuid;

use crate::error::ApiError;

/// Generate CalDAV multistatus response for PROPFIND
pub fn generate_propfind_multistatus(
    user_id: Uuid,
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
    writer
        .write_event(Event::Start(multistatus))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // Calendar collection response
    write_calendar_response(&mut writer, user_id, calendar)?;

    // Event responses (only for Depth: 1)
    if depth == "1" {
        for event in events {
            write_event_response(&mut writer, user_id, event)?;
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
    user_id: Uuid,
    calendar: &Calendar,
) -> Result<(), ApiError> {
    // <response>
    writer
        .write_event(Event::Start(BytesStart::new("d:response")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <href>/caldav/{user_id}/</href>
    writer
        .write_event(Event::Start(BytesStart::new("d:href")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(&format!(
            "/caldav/{}/",
            user_id
        ))))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:href")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <propstat>
    writer
        .write_event(Event::Start(BytesStart::new("d:propstat")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <prop>
    writer
        .write_event(Event::Start(BytesStart::new("d:prop")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <resourcetype><collection/><calendar/></resourcetype>
    writer
        .write_event(Event::Start(BytesStart::new("d:resourcetype")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Empty(BytesStart::new("d:collection")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Empty(BytesStart::new("cal:calendar")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:resourcetype")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <displayname>
    writer
        .write_event(Event::Start(BytesStart::new("d:displayname")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(&calendar.name)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:displayname")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <getctag>
    writer
        .write_event(Event::Start(BytesStart::new("cal:getctag")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(&calendar.ctag)))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("cal:getctag")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <supported-calendar-component-set>
    writer
        .write_event(Event::Start(BytesStart::new(
            "cal:supported-calendar-component-set",
        )))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    let mut comp = BytesStart::new("cal:comp");
    comp.push_attribute(("name", "VEVENT"));
    writer
        .write_event(Event::Empty(comp))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new(
            "cal:supported-calendar-component-set",
        )))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // </prop>
    writer
        .write_event(Event::End(BytesEnd::new("d:prop")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <status>HTTP/1.1 200 OK</status>
    writer
        .write_event(Event::Start(BytesStart::new("d:status")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new("HTTP/1.1 200 OK")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:status")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // </propstat>
    writer
        .write_event(Event::End(BytesEnd::new("d:propstat")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // </response>
    writer
        .write_event(Event::End(BytesEnd::new("d:response")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    Ok(())
}

/// Write event resource response
fn write_event_response(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    user_id: Uuid,
    event: &CalEvent,
) -> Result<(), ApiError> {
    // <response>
    writer
        .write_event(Event::Start(BytesStart::new("d:response")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <href>/caldav/{user_id}/{uid}.ics</href>
    writer
        .write_event(Event::Start(BytesStart::new("d:href")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(&format!(
            "/caldav/{}/{}.ics",
            user_id, event.uid
        ))))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:href")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <propstat>
    writer
        .write_event(Event::Start(BytesStart::new("d:propstat")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <prop>
    writer
        .write_event(Event::Start(BytesStart::new("d:prop")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <getetag>
    writer
        .write_event(Event::Start(BytesStart::new("d:getetag")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(&format!("\"{}\"", event.etag))))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:getetag")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <getcontenttype>
    writer
        .write_event(Event::Start(BytesStart::new("d:getcontenttype")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new("text/calendar; charset=utf-8")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:getcontenttype")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // </prop>
    writer
        .write_event(Event::End(BytesEnd::new("d:prop")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // <status>HTTP/1.1 200 OK</status>
    writer
        .write_event(Event::Start(BytesStart::new("d:status")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new("HTTP/1.1 200 OK")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("d:status")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // </propstat>
    writer
        .write_event(Event::End(BytesEnd::new("d:propstat")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    // </response>
    writer
        .write_event(Event::End(BytesEnd::new("d:response")))
        .map_err(|e| ApiError::Internal(format!("XML write error: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use televent_core::models::EventStatus;

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

        let xml = generate_propfind_multistatus(user_id, &calendar, &[], "0").unwrap();

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
            timezone: "UTC".to_string(),
            created_at: now,
            updated_at: now,
        };

        let xml = generate_propfind_multistatus(user_id, &calendar, &[event], "1").unwrap();

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

        let xml = generate_propfind_multistatus(user_id, &calendar, &[], "0").unwrap();

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
}
