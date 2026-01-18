# CalDAV Compliance

## Supported RFC Standards

- **RFC 4791**: CalDAV (Calendaring Extensions to WebDAV)
- **RFC 5545**: iCalendar format
- **RFC 6578**: sync-collection (Calendar Collection Synchronization)

## Implemented Methods

| Method   | Endpoint                            | Status | Notes                          |
| -------- | ----------------------------------- | ------ | ------------------------------ |
| OPTIONS  | `/caldav/`                          | TODO   | Capabilities discovery         |
| PROPFIND | `/caldav/{user_id}/`                | TODO   | Calendar metadata              |
| PROPFIND | `/caldav/{user_id}/calendar.ics`    | TODO   | Event listing                  |
| REPORT   | `/caldav/{user_id}/`                | TODO   | calendar-query, sync-collection|
| GET      | `/caldav/{user_id}/{event_uid}.ics` | TODO   | Fetch single event             |
| PUT      | `/caldav/{user_id}/{event_uid}.ics` | TODO   | Create/update event            |
| DELETE   | `/caldav/{user_id}/{event_uid}.ics` | TODO   | Delete event                   |

## Key Implementation Details

### Sync Token Strategy
- Increment `calendar.sync_token` on any change
- Use Postgres sequence for atomic increments
- Return changed/deleted events since token

### ETag/CTag
- **ETag**: SHA256 hash of serialized event (NOT timestamp)
- **CTag**: Timestamp of last calendar change
- Used for conflict detection and caching

### Conflict Handling
- Client sends `If-Match: <etag>` header on PUT
- Return `412 Precondition Failed` if etag doesn't match
- Client must fetch latest version and merge changes

### Recurrence Rules
- Store RRULE in database as string
- Expand instances in queries (not at creation time)
- Handle EXDATE for exceptions

## Testing

### caldav-tester Suite

```bash
git clone https://github.com/apple/ccs-caldavtester
cd ccs-caldavtester
python testcaldav.py --server localhost:3000 --all
```

Target: 100% pass rate

## Tested Clients

- [ ] Apple Calendar (iOS)
- [ ] Apple Calendar (macOS)
- [ ] Thunderbird
- [ ] DAVx⁵ (Android)
- [ ] Evolution
- [ ] CalDAV-Sync (Android)

## Known Limitations

1. No shared calendars (one calendar per user)
2. No attachments support
3. No alarm/reminder sync via CalDAV (handled via bot)
4. No calendar colors sync (stored locally only)

## Future Enhancements

- Support for multiple calendars per user
- Calendar sharing (read-only, read-write)
- Attachment support
- Advanced recurrence patterns (RDATE, complex RRULE)
