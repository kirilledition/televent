<objective>
Implement CalDAV write operations (PUT for create/update, DELETE for removal) so users can create and modify events directly from calendar clients like Thunderbird.

After completing this prompt, the full CalDAV sync flow will work:
- Events created in Telegram appear in Thunderbird
- Events created in Thunderbird appear in Telegram bot (/today, /tomorrow)
- Events can be modified from either interface
- Events can be deleted from either interface
</objective>

<context>
Read the project conventions first:
@CLAUDE.md

Current CalDAV implementation:
@crates/api/src/routes/caldav.rs - Route handlers (GET/PROPFIND/REPORT exist)
@crates/api/src/routes/caldav_xml.rs - XML and iCalendar handling
@crates/core/src/models.rs - Event model
@crates/api/src/db/events.rs - Database operations

RFC References:
- RFC 4791 Section 5.3.2: Creating Calendar Object Resources (PUT)
- RFC 4791 Section 5.3.3: Deleting Calendar Object Resources (DELETE)
- RFC 5545: iCalendar parsing requirements

Database schema (events table):
- id (UUID), calendar_id, uid, title, description, location
- start_time, end_time (timestamptz, store as UTC)
- version (integer, increment on update)
- created_at, updated_at
</context>

<requirements>
**1. PUT Handler - Create New Event**
When PUT to `/caldav/{user_id}/calendars/{calendar_id}/{event_uid}.ics`:
- If event with UID doesn't exist → Create new event
- Parse iCalendar body to extract: SUMMARY, DTSTART, DTEND, LOCATION, DESCRIPTION
- Convert timestamps to UTC
- Generate new UUID for database id
- Use UID from iCalendar as the `uid` field
- Return 201 Created with ETag header
- Update calendar's ctag

**2. PUT Handler - Update Existing Event**
When PUT with `If-Match` header:
- Verify ETag matches current event version
- If mismatch → Return 412 Precondition Failed
- Update event fields from iCalendar body
- Increment version field
- Return 200 OK (or 204 No Content) with new ETag
- Update calendar's ctag

**3. DELETE Handler**
When DELETE to `/caldav/{user_id}/calendars/{calendar_id}/{event_uid}.ics`:
- Verify event exists and belongs to calendar
- Optional: Check `If-Match` header for ETag
- Delete event from database
- Return 204 No Content
- Update calendar's ctag

**4. ETag Generation**
- ETag = quoted SHA256 hash of event's serialized iCalendar
- Or simpler: ETag = `"v{version}"` where version is the DB version field
- Must be consistent: same event state → same ETag

**5. iCalendar Parsing**
Create parser to extract event data from VCALENDAR/VEVENT:
- Use `icalendar` crate or manual parsing
- Handle both UTC timestamps (YYYYMMDDTHHMMSSZ) and local with TZID
- Convert all times to UTC for storage
- Extract: UID, SUMMARY, DTSTART, DTEND, LOCATION, DESCRIPTION, SEQUENCE
- Validate required fields present
</requirements>

<implementation>
Files to create/modify:

1. `crates/api/src/routes/caldav.rs`
   - Add PUT handler: `put_calendar_object`
   - Add DELETE handler: `delete_calendar_object`
   - Route setup: `.route("/:user_id/calendars/:calendar_id/:event_uid.ics", put(put_calendar_object).delete(delete_calendar_object))`

2. `crates/api/src/routes/caldav_ical.rs` (new file)
   - iCalendar parser: `parse_vcalendar(ical_string: &str) -> Result<EventData, CalendarError>`
   - EventData struct with optional fields matching VEVENT properties
   - Handle timestamp parsing with chrono

3. `crates/api/src/db/events.rs`
   - Add `create_event_from_caldav()` - insert with UID
   - Add `update_event_from_caldav()` - update by UID, increment version
   - Add `delete_event_by_uid()` - soft or hard delete
   - Add `get_event_by_uid()` - fetch by UID for ETag check

4. `crates/api/src/db/calendars.rs`
   - Add `increment_ctag()` - atomic increment of calendar sync tag

Error handling (use typed errors, not unwrap):
```rust
#[derive(Error, Debug)]
pub enum CalDavError {
    #[error("Event not found: {0}")]
    EventNotFound(String),
    #[error("ETag mismatch: expected {expected}, got {actual}")]
    ETagMismatch { expected: String, actual: String },
    #[error("Invalid iCalendar: {0}")]
    InvalidICalendar(String),
}
```

Response headers for PUT/DELETE:
- `ETag`: Current event version (for PUT success)
- `Content-Length: 0` (for 204 responses)
</implementation>

<output>
Create/modify these files:
- `./crates/api/src/routes/caldav.rs` - PUT and DELETE handlers
- `./crates/api/src/routes/caldav_ical.rs` - iCalendar parser (new)
- `./crates/api/src/routes/mod.rs` - Export new module
- `./crates/api/src/db/events.rs` - New database operations

Add dependency if needed:
```bash
cargo add --package api icalendar
```
</output>

<verification>
Test the complete CRUD flow:

**1. Create event from Thunderbird:**
```bash
# Watch API logs
CALDAV_DEBUG=1 cargo run --package api
# In Thunderbird: Create new event, save
# Verify in DB:
psql -c "SELECT uid, title, start_time FROM events ORDER BY created_at DESC LIMIT 1"
```

**2. Update event from Thunderbird:**
```bash
# In Thunderbird: Edit event title or time, save
# Verify in DB that version incremented:
psql -c "SELECT uid, title, version FROM events WHERE uid = '{uid}'"
```

**3. Delete event from Thunderbird:**
```bash
# In Thunderbird: Delete event
# Verify removed from DB:
psql -c "SELECT COUNT(*) FROM events WHERE uid = '{uid}'"
```

**4. Verify Telegram bot sees changes:**
```bash
# In Telegram: /today or /tomorrow
# Events created/modified in Thunderbird should appear
```

**5. Verify curl operations:**
```bash
# Create
curl -X PUT http://localhost:3000/caldav/{id}/calendars/default/test-event.ics \
  -u "{id}:{password}" \
  -H "Content-Type: text/calendar" \
  -d "BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test-event
DTSTAMP:20260124T120000Z
DTSTART:20260125T100000Z
DTEND:20260125T110000Z
SUMMARY:Test Event
END:VEVENT
END:VCALENDAR"

# Update (with ETag)
curl -X PUT http://localhost:3000/caldav/{id}/calendars/default/test-event.ics \
  -u "{id}:{password}" \
  -H "Content-Type: text/calendar" \
  -H 'If-Match: "v1"' \
  -d "..." # Updated VCALENDAR

# Delete
curl -X DELETE http://localhost:3000/caldav/{id}/calendars/default/test-event.ics \
  -u "{id}:{password}"
```
</verification>

<success_criteria>
- PUT creates new events (returns 201 with ETag)
- PUT updates existing events when If-Match header present (returns 200/204)
- PUT returns 412 on ETag mismatch (conflict detection works)
- DELETE removes events (returns 204)
- Calendar ctag updates on any change (enables sync detection)
- Thunderbird can create, edit, and delete events
- Events sync bidirectionally between Telegram bot and CalDAV clients
- No unwrap() or expect() - proper error handling throughout
- Code compiles without clippy warnings
</success_criteria>
