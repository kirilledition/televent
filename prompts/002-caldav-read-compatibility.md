<objective>
Fix CalDAV read operations to achieve compatibility with GUI calendar clients (Thunderbird, Evolution).

Events exist in the database and are returned correctly via curl, but GUI clients show "calendar unavailable" or fail to display events. Use the CalDAV debug logging (from prompt 001) to diagnose and fix issues iteratively.
</objective>

<context>
Read the project conventions first:
@CLAUDE.md

CalDAV implementation:
@crates/api/src/routes/caldav.rs - Route handlers
@crates/api/src/routes/caldav_xml.rs - XML response builders
@docs/MVP_BLOCKERS_AND_PLAN.md - Known issues and potential causes

Key CalDAV RFCs:
- RFC 4791: CalDAV specification
- RFC 5545: iCalendar format
- RFC 6578: Collection synchronization (sync-token)

Current working state:
- ✅ PROPFIND returns XML with event list
- ✅ REPORT calendar-query returns iCalendar data
- ✅ HTTP Basic Auth working
- ✅ curl/cadaver work correctly
- ❌ Thunderbird shows "calendar unavailable"
</context>

<requirements>
Apply iterative fixes, testing after each change:

**Round 1: XML Namespace and Structure**
1. Verify XML namespace declarations match RFC 4791 exactly:
   - `DAV:` namespace for WebDAV properties
   - `urn:ietf:params:xml:ns:caldav` for CalDAV properties
   - `http://calendarserver.org/ns/` for calendar-server extensions

2. Check multistatus response structure:
   - Each `<response>` must have `<href>` and `<propstat>`
   - `<propstat>` must have `<prop>` and `<status>`
   - Status must be exactly `HTTP/1.1 200 OK` or `HTTP/1.1 404 Not Found`

**Round 2: Required Calendar Properties**
Ensure PROPFIND on calendar collection returns ALL required properties:
- `displayname` - Human-readable name
- `resourcetype` - Must contain `<calendar/>` and `<collection/>`
- `getctag` - Calendar change tag (for sync)
- `supported-calendar-component-set` - Must include `VEVENT`
- `calendar-home-set` - URL to user's calendar home
- `current-user-principal` - URL to principal resource
- `owner` - Owner principal URL
- `supported-report-set` - List of supported REPORTs

**Round 3: iCalendar Format Compliance**
Verify event iCalendar output is strictly RFC 5545 compliant:
- VCALENDAR wrapper with VERSION:2.0 and PRODID
- VEVENT with required properties: UID, DTSTAMP, DTSTART
- DTEND or DURATION (one must be present for timed events)
- All timestamps in UTC format (YYYYMMDDTHHMMSSZ)
- Proper line folding (lines >75 chars folded with CRLF + space)
- CRLF line endings throughout

**Round 4: Content-Type Headers**
Ensure correct Content-Type for all responses:
- PROPFIND/REPORT: `application/xml; charset=utf-8`
- GET on .ics: `text/calendar; charset=utf-8`
- Include `Content-Length` header

**Round 5: ETag and Sync Support**
- Every event resource must have `getetag` property
- ETag format: quoted string, e.g., `"abc123"`
- Calendar collection must have `getctag`
- Implement `sync-collection` REPORT (RFC 6578)
</requirements>

<implementation>
Iterative process for each round:

1. Run API with CalDAV debug logging enabled:
   ```bash
   CALDAV_DEBUG=1 cargo run --package api
   ```

2. Test with Thunderbird (add calendar, watch logs)

3. Compare our response to RFC requirements

4. Make targeted fix in `caldav_xml.rs` or `caldav.rs`

5. Test again - if working, commit; if not, continue to next issue

Key files to modify:
- `crates/api/src/routes/caldav_xml.rs` - XML builders, iCalendar serialization
- `crates/api/src/routes/caldav.rs` - Request handlers, response headers

Testing sequence:
1. Clear Thunderbird calendar cache
2. Add calendar: `http://localhost:3000/caldav/{telegram_id}/calendars/default`
3. Watch API logs for request sequence
4. Check Thunderbird error console (Tools > Developer Tools > Error Console)
</implementation>

<output>
Modify these files:
- `./crates/api/src/routes/caldav_xml.rs` - Fix XML/iCalendar output
- `./crates/api/src/routes/caldav.rs` - Fix headers and handlers

Document findings in:
- `./docs/caldav_client_compatibility.md` - What changes were needed and why
</output>

<verification>
Test with multiple clients after each round:

1. **curl baseline** (should keep working):
   ```bash
   curl -X PROPFIND http://localhost:3000/caldav/{id}/calendars/default \
     -H "Depth: 1" -u "{id}:{password}"
   ```

2. **Thunderbird**:
   - Add network calendar: CalDAV
   - URL: `http://localhost:3000/caldav/{telegram_id}/calendars/default`
   - Events should appear within 30 seconds

3. **Evolution** (if available):
   - New calendar > On the Web > CalDAV
   - Verify events display

4. **Validate iCalendar** output:
   ```bash
   # Fetch event and validate
   curl http://localhost:3000/caldav/{id}/calendars/default/{event}.ics \
     -u "{id}:{password}" | python3 -c "import icalendar, sys; icalendar.Calendar.from_ical(sys.stdin.read())"
   ```
</verification>

<success_criteria>
- Thunderbird displays all events without "calendar unavailable" warning
- Evolution (or another client) also displays events
- curl continues to work as baseline
- iCalendar output passes validation
- All required CalDAV properties present in PROPFIND responses
- Commit after each successful fix round
</success_criteria>
