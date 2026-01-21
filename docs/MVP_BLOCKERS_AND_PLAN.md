# MVP Blockers and Action Plan

**Last Updated**: 2026-01-21
**Status**: 2 Critical Blockers Identified

## Current State

### ✅ What's Working

1. **Database Infrastructure**
   - PostgreSQL with proper schema
   - Migrations running successfully
   - Users, calendars, events, device_passwords tables working

2. **API Server**
   - Running on port 3000
   - Event creation via POST `/api/events` - ✅ WORKS
   - Event retrieval via GET `/api/events` - ✅ WORKS
   - CalDAV endpoints responding - ✅ WORKS
   - Authentication with device passwords - ✅ WORKS

3. **Telegram Bot**
   - Bot running and responding to commands
   - `/start` - creates user and calendar ✅
   - `/today`, `/tomorrow`, `/week` - show events ✅
   - `/device add` - creates device passwords ✅
   - `/device list` - lists devices ✅
   - All commands working correctly

4. **CalDAV Server (Partial)**
   - PROPFIND responds with proper XML ✅
   - REPORT calendar-query returns events ✅
   - HTTP Basic Auth working ✅
   - curl/cadaver can fetch events ✅
   - Returns proper iCalendar format ✅
   - WWW-Authenticate header present ✅
   - getlastmodified property present ✅

### ❌ Critical Blockers for MVP

## BLOCKER #1: Bot Event Creation Not Implemented

**Problem**: The `/create` command only shows instructions. There's no actual event creation logic in the bot.

**Current Behavior**:
```
User: /create
Bot: "To create an event, send a message in this format: ..."
User: <sends event details>
Bot: <nothing happens>
```

**Root Cause**:
- File: `crates/bot/src/main.rs:54`
- The bot only handles commands (messages starting with `/`)
- No message handler for non-command text input
- No event parsing logic implemented

**Impact**: Users cannot create events via Telegram bot (core functionality missing)

---

## BLOCKER #2: CalDAV Client Compatibility Issues

**Problem**: Thunderbird (and likely other CalDAV clients) can subscribe to calendar but doesn't display events.

**Current Behavior**:
- Thunderbird subscribes successfully
- Shows "calendar unavailable" warning
- No events displayed
- No errors in API logs
- curl/cadaver work perfectly with same credentials

**What We Know**:
- ✅ Authentication works (401/207 responses correct)
- ✅ PROPFIND returns proper XML with all events listed
- ✅ REPORT calendar-query returns proper iCalendar data
- ✅ Individual event GET requests work
- ✅ All required CalDAV properties present (displayname, resourcetype, getctag, getlastmodified)
- ❌ Thunderbird doesn't show events despite successful responses

**Potential Root Causes**:
1. Missing CalDAV properties that Thunderbird specifically requires
2. XML namespace issues
3. iCalendar format not strict enough for GUI clients
4. Calendar synchronization token (ctag) not updating properly
5. Thunderbird caching issues

**Impact**: Cannot use standard calendar clients (Thunderbird, Apple Calendar, etc.)

---

## Action Plan

### Phase 1: Fix Bot Event Creation (Priority: P0)

**Estimated Effort**: 2-4 hours

#### Step 1.1: Implement Message Handler
**File**: `crates/bot/src/main.rs`

Add a message handler alongside the command handler:
```rust
// After Command::repl, add message handler
teloxide::repl(bot, |bot: Bot, msg: Message| async move {
    if msg.text().is_some() && !msg.text().unwrap().starts_with('/') {
        handle_text_message(bot, msg, db).await?;
    }
    Ok(())
})
```

#### Step 1.2: Create Event Parser
**File**: `crates/bot/src/event_parser.rs` (new file)

Implement parsing logic:
- Parse event title (line 1)
- Parse datetime (line 2): `YYYY-MM-DD HH:MM`
- Parse duration (line 3, optional): integer minutes
- Parse location (line 4, optional)
- Return structured event data or error

Use `chrono-english` crate for natural language datetime parsing (already in dependencies).

#### Step 1.3: Implement Event Creation Handler
**File**: `crates/bot/src/handlers.rs`

Add `handle_text_message`:
- Check if user has a calendar
- Parse message into event data
- Generate unique UID (UUID format)
- Calculate end time from duration
- Call `db::events::create_event()`
- Send confirmation message to user

#### Step 1.4: Add Conversation State Management
**File**: `crates/bot/src/state.rs` (new file)

Track user state:
- Is user creating an event?
- What step are they on?
- Use HashMap<ChatId, ConversationState>

Alternative (simpler): Just parse any non-command message as potential event.

#### Step 1.5: Testing
```bash
# In Telegram:
/create
# Bot shows template

# Send:
Team Standup
2026-01-22 10:00
30
Office

# Expected: Bot creates event and confirms
# Verify with: /tomorrow
```

**Success Criteria**:
- User can create events via Telegram
- Events appear in database
- Events show up in `/today`, `/tomorrow`, `/week`
- Events sync to CalDAV

---

### Phase 2: Fix CalDAV Client Compatibility (Priority: P0)

**Estimated Effort**: 4-8 hours

#### Step 2.1: Debug Thunderbird CalDAV Requests
**Approach**: Add verbose logging to see exact requests Thunderbird makes

**File**: `crates/api/src/middleware/caldav_auth.rs` and `crates/api/src/routes/caldav.rs`

Add logging:
```rust
tracing::debug!("CalDAV Request: {} {}", method, uri);
tracing::debug!("Headers: {:?}", headers);
tracing::debug!("Body: {}", body_str);
```

**Action**:
1. Clear Thunderbird cache: `rm -rf ~/.thunderbird/*/calendar-data/*`
2. Re-add calendar in Thunderbird
3. Watch API logs carefully
4. Document exact sequence of requests

#### Step 2.2: Compare with Working CalDAV Server
**Tools**: Radicale (reference CalDAV implementation)

```bash
# Install Radicale
pip install radicale
radicale --config ""

# Add to Thunderbird
# Compare PROPFIND/REPORT responses with ours
```

**Check**:
- XML structure differences
- Property names and namespaces
- Response ordering
- Status codes in propstat elements

#### Step 2.3: Validate iCalendar Format
**Tool**: `icalendar` validator

Check if our VEVENT format is fully RFC 5545 compliant:
- Required properties present (DTSTAMP, DTSTART, DTEND, UID)
- Property value formats correct
- Timezone handling
- SEQUENCE property behavior

**File**: `crates/api/src/routes/caldav_xml.rs` - `to_icalendar()` function

#### Step 2.4: Fix Calendar-Level Properties
**Potential Missing Properties**:

Check if we need to add:
- `<calendar-color>` - GUI clients often require this
- `<calendar-order>` - Display ordering
- `<calendar-timezone>` - Default timezone
- `<current-user-privilege-set>` - Permissions
- `<owner>` - Calendar owner

**File**: `crates/api/src/routes/caldav_xml.rs` - `write_calendar_response()`

#### Step 2.5: Implement Proper Sync-Token Support
**Current Issue**: We return `getctag` but may not be using sync-token correctly

**RFC 6578 Requirements**:
- Support `<sync-collection>` REPORT
- Return `<sync-token>` in responses
- Increment token on any change

**Files**:
- Database: Add `sync_token` to calendars table
- `crates/api/src/routes/caldav.rs`: Implement sync-collection handler

#### Step 2.6: Test with Multiple Clients
**Clients to Test**:
1. Thunderbird (Linux)
2. Evolution (Linux alternative)
3. curl (baseline)
4. cadaver (baseline)

Document which work and which don't.

#### Step 2.7: Enable CalDAV Debug Mode in Thunderbird
```bash
# In Thunderbird, open Config Editor (about:config)
# Set these preferences:
calendar.debug.log = true
calendar.debug.log.verbose = true

# Check logs at:
~/.thunderbird/*/calendar-data/cache.sqlite
```

**Success Criteria**:
- Thunderbird displays all 3 events
- Events can be edited in Thunderbird
- Changes sync back to server
- No "calendar unavailable" warnings

---

## Testing Strategy

### After Fixing Blocker #1 (Bot Event Creation)
```bash
# 1. Create event via bot
# Telegram: /create → send event details

# 2. Verify in database
./scripts/tests/test_verify_events.sh

# 3. Verify in bot
# Telegram: /today

# 4. Verify in API
curl http://localhost:3000/api/events

# 5. Verify in CalDAV
./scripts/tests/test_thunderbird_caldav.sh
```

### After Fixing Blocker #2 (CalDAV Compatibility)
```bash
# 1. Remove and re-add calendar in Thunderbird
# 2. Wait 10 seconds for sync
# 3. Verify events appear
# 4. Create event in Thunderbird
# 5. Verify it appears in bot /today
# 6. Edit event in bot (future feature)
# 7. Verify change appears in Thunderbird
```

---

## Alternative Approaches

### For Blocker #1: Simpler Bot UI
Instead of multi-line parsing, use inline format:
```
/newevent Team Meeting @ 2026-01-22 10:00 for 30min at Office
```

**Pros**: Simpler parsing, one command
**Cons**: Less user-friendly, harder to type

### For Blocker #2: Use Existing Library
Consider using `caldav` Rust crate instead of custom implementation:
- https://crates.io/crates/caldav

**Pros**: Battle-tested, RFC-compliant
**Cons**: May not fit our architecture, learning curve

---

## Success Metrics for MVP

After fixing both blockers, we should be able to:

1. ✅ User installs Telegram bot
2. ✅ User creates events via /create
3. ✅ Events appear in bot (/today, /tomorrow, /week)
4. ✅ User generates CalDAV password (/device add)
5. ✅ User adds calendar to Thunderbird
6. ✅ Events sync to Thunderbird automatically
7. ✅ User can see events in both interfaces
8. ⏳ User can invite others (Phase 2 - Interceptor)

---

## Timeline Estimate

| Task | Effort | Dependencies |
|------|--------|--------------|
| Bot event creation | 4 hours | None |
| Bot testing | 1 hour | Event creation |
| CalDAV debugging | 2 hours | None |
| CalDAV fixes | 4 hours | Debugging |
| CalDAV testing | 2 hours | Fixes |
| **Total** | **13 hours** | Sequential |

**Realistic Timeline**: 2-3 days of focused work

---

## Next Immediate Steps

1. **Start with Blocker #1** (easier, clear implementation)
   - Create `crates/bot/src/event_parser.rs`
   - Implement message handler
   - Test event creation

2. **Then tackle Blocker #2** (harder, needs debugging)
   - Enable verbose CalDAV logging
   - Compare with Radicale
   - Fix identified issues

3. **Iterate until both work**
   - Test frequently
   - Commit working states
   - Document findings

---

## Resources

### Documentation
- RFC 4791: CalDAV Specification
- RFC 5545: iCalendar Format
- RFC 6578: CalDAV Sync
- Teloxide Bot Examples: https://github.com/teloxide/teloxide/tree/master/examples

### Reference Implementations
- Radicale: https://github.com/Kozea/Radicale (Python CalDAV server)
- Baikal: https://github.com/sabre-io/Baikal (PHP CalDAV server)

### Tools
- `xmllint` - Validate XML responses
- `cadaver` - Test CalDAV from terminal
- Thunderbird config editor - Debug calendar sync

---

## Appendix: Current System Status

### Database
```sql
-- 1 user
SELECT COUNT(*) FROM users; -- 1

-- 1 calendar
SELECT COUNT(*) FROM calendars; -- 1

-- 3 events (created via API, not bot)
SELECT COUNT(*) FROM events; -- 3

-- 1 device password
SELECT COUNT(*) FROM device_passwords; -- 1
```

### Services Running
- ✅ PostgreSQL on port 5432
- ✅ API server on port 3000
- ✅ Bot connected to Telegram
- ✅ Worker processing outbox (no jobs currently)

### Test Scripts Available
- `scripts/tests/test_create_events.sh` - Create events via API
- `scripts/tests/test_verify_events.sh` - Check database
- `scripts/tests/test_cleanup_events.sh` - Remove test data
- `scripts/tests/test_thunderbird_caldav.sh` - CalDAV compatibility test
- `scripts/utils/get_caldav_url.sh` - Get credentials

---

**Document Owner**: Development Team
**Review Frequency**: After each blocker is fixed
**Next Review**: After implementing bot event creation
