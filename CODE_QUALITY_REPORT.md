# Code Quality Review Report - Televent

**Date:** 2026-01-20
**Reviewer:** Automated Code Quality Review
**Repository:** kirilledition/televent

## Executive Summary

This report documents a comprehensive code quality review of the Televent CalDAV/Telegram bot project. The review focused on identifying bugs, contradictions, missing functionality, security issues, and violations of the project's zero-tolerance coding standards.

**Overall Assessment:** The codebase is generally well-structured with good error handling practices. However, there are **3 critical issues** and several **high-priority improvements** needed before production deployment.

## Critical Issues (Must Fix)

### 1. Type Safety Violation - Raw Uuid Usage Throughout Codebase
**Severity:** 🔴 CRITICAL  
**Location:** `crates/core/src/models.rs` (lines 9-146) and all dependent code  
**Rule Violated:** Zero-tolerance rule #3: "Use newtypes (UserId(Uuid), not raw Uuid)"

**Problem:**  
All models use raw `Uuid` types instead of newtypes. This violates the stated zero-tolerance rule for type safety and creates a high risk of ID confusion bugs (e.g., passing a `calendar_id` where a `user_id` is expected).

**Evidence:**
```rust
// Current (INCORRECT):
pub struct User {
    pub id: Uuid,
    pub telegram_id: i64,
    // ...
}

pub struct Calendar {
    pub id: Uuid,
    pub user_id: Uuid,  // Can accidentally pass calendar_id here!
    // ...
}
```

**Impact:**  
- Type confusion can lead to authorization bypasses (e.g., accessing another user's calendar)
- CalDAV spec has complex ID requirements - mixing IDs causes sync corruption
- Violates project's own documented safety requirements

**Recommendation:**
```rust
// Define newtypes:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct EventId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct CalendarId(pub Uuid);

// Then update all models to use these types
```

**Effort:** High (affects ~40+ files)  
**Priority:** P0 - Must fix before production

---

### 2. Missing Optimistic Locking in update_event
**Severity:** 🔴 CRITICAL  
**Location:** `crates/api/src/db/events.rs:238`  
**Rule Violated:** CalDAV RFC 4791 requirements, data integrity

**Problem:**  
The `update_event` function increments the version column but doesn't check it in the WHERE clause. This allows lost updates when two CalDAV clients update the same event simultaneously.

**Evidence:**
```rust
// Line 238 (INCORRECT):
WHERE id = $1
RETURNING *

// Should be:
WHERE id = $1 AND version = $2
RETURNING *
```

**Impact:**  
- Lost updates in concurrent scenarios
- CalDAV sync corruption
- Users lose edits made from different devices

**Test Case to Reproduce:**
1. Client A: GET event (version=1, etag=abc)
2. Client B: GET event (version=1, etag=abc)
3. Client A: PUT event with If-Match: abc (succeeds, version=2)
4. Client B: PUT event with If-Match: abc (should fail but succeeds, version=3, overwrites Client A's changes)

**Recommendation:**
```rust
pub async fn update_event(
    pool: &PgPool,
    event_id: Uuid,
    current_version: i32,  // ADD THIS PARAMETER
    summary: Option<String>,
    description: Option<Option<String>>,
    location: Option<Option<String>>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    is_all_day: Option<bool>,
    status: Option<EventStatus>,
    rrule: Option<Option<String>>,
    etag: String,
) -> Result<Event, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        UPDATE events SET
            summary = COALESCE($2, summary),
            description = COALESCE($3, description),
            location = COALESCE($4, location),
            start = COALESCE($5, start),
            "end" = COALESCE($6, "end"),
            is_all_day = COALESCE($7, is_all_day),
            status = COALESCE($8, status),
            rrule = COALESCE($9, rrule),
            version = version + 1,
            etag = $10,
            updated_at = NOW()
        WHERE id = $1 AND version = $11  -- ADD VERSION CHECK
        RETURNING *
        "#,
    )
    .bind(event_id)        // $1
    .bind(summary)         // $2
    .bind(description)     // $3
    .bind(location)        // $4
    .bind(start)           // $5
    .bind(end)             // $6
    .bind(is_all_day)      // $7
    .bind(status)          // $8
    .bind(rrule)           // $9
    .bind(etag)            // $10
    .bind(current_version) // $11 - ADD THIS BIND
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| sqlx::Error::RowNotFound)  // Handle version mismatch
}
```

**Effort:** Medium  
**Priority:** P0 - Must fix before production

---

### 3. Race Condition in CalDAV PUT Operation
**Severity:** 🟠 HIGH  
**Location:** `crates/api/src/routes/caldav.rs:154-207`  
**Rule Violated:** TOCTOU (Time-Of-Check-Time-Of-Use) vulnerability

**Problem:**  
The PUT operation has a race condition between checking if an event exists (line 155) and updating/creating it (lines 173/189). Two concurrent PUT requests could both see no existing event and both attempt to create, causing a unique constraint violation.

**Evidence:**
```rust
// Lines 154-204:
// 1. Check if event exists
let existing = db::events::get_event_by_uid(&pool, calendar.id, &uid).await?;

// 2. Race window here! Another request can create the event between check and update
let (status_code, etag) = if let Some(existing_event) = existing {
    // Update path
    // ...
} else {
    // Create path
    // ...
};
```

**Impact:**  
- Duplicate key errors in production logs
- Failed sync operations
- Poor user experience

**Recommendation:**
```sql
-- Use database-level upsert:
INSERT INTO events (calendar_id, uid, summary, ...)
VALUES ($1, $2, $3, ...)
ON CONFLICT (calendar_id, uid) DO UPDATE SET
    summary = EXCLUDED.summary,
    version = events.version + 1,
    ...
RETURNING *, xmax::text::int > 0 AS was_updated;
```

Or use `SELECT FOR UPDATE`:
```rust
let existing = sqlx::query_as::<_, Event>(
    "SELECT * FROM events WHERE calendar_id = $1 AND uid = $2 FOR UPDATE"
)
.bind(calendar.id)
.bind(&uid)
.fetch_optional(&pool)
.await?;
```

**Effort:** Medium  
**Priority:** P1 - Fix before beta

---

## High-Priority Issues

### 4. Dead Code in Bot Crate
**Severity:** 🟡 MEDIUM  
**Location:** `crates/bot/src/db.rs`

**Problems:**
- `UserInfo.id` and `UserInfo.telegram_username` fields never read (lines 62-64)
- `EventInfo.id`, `EventInfo.end`, `EventInfo.user_id` never read (lines 70-75)
- `AttendeeInfo` struct never constructed (line 90)
- `get_event_attendees()` method never called (line 409)

**Impact:**  
- Code bloat
- Maintenance burden
- Fails `cargo clippy -- -D warnings`

**Recommendation:**  
Remove unused code or mark with `#[allow(dead_code)]` if planned for future use.

**Priority:** P2 - Fix in next sprint

---

### 5. Needless Question Mark Operators
**Severity:** 🟡 MEDIUM  
**Location:** `crates/bot/src/db.rs:489, 518`

**Problems:**
```rust
// Line 489 (inefficient):
Ok(row.try_get("id")?)

// Should be:
row.try_get("id")
```

**Impact:**  
- Code verbosity
- Clippy warnings

**Recommendation:**  
Remove the enclosing `Ok()` and `?` operator as suggested by clippy.

**Priority:** P2 - Low effort fix

---

## Code Quality Observations

### Positive Findings ✅

1. **No println!() Violations** - All logging uses `tracing::info!`, `tracing::error!`, etc.
2. **Good Error Handling** - Proper use of `thiserror` for libraries and `anyhow` for binaries
3. **Most unwrap() in Tests Only** - Production code mostly uses `?` propagation
4. **Proper Async Runtime** - Consistent use of tokio throughout
5. **Good Documentation** - Most modules have doc comments
6. **Comprehensive Test Coverage** - Unit tests for core logic, integration tests for CalDAV

### Acceptable Patterns ✅

The following patterns are acceptable and do NOT need to be changed:

1. **unwrap_or() / unwrap_or_else()** - These are safe alternatives to unwrap():
   - `crates/api/src/routes/caldav.rs:56` - `unwrap_or("0")` with default value
   - `crates/api/src/routes/ical.rs:134` - `unwrap_or(false)` with default value
   - `crates/api/src/routes/ical.rs:140` - `unwrap_or(tz_part.len())` with fallback
   - `crates/bot/src/main.rs:27` - `unwrap_or_else()` with fallback config

2. **unwrap() in Tests** - All test code can safely use unwrap():
   - All files under `/tests/` directories
   - All `#[test]` functions
   - All `#[cfg(test)]` modules

---

## Security Considerations

### CalDAV Compliance ✅

The code generally follows CalDAV RFC requirements:

1. **ETag Handling** ✅ - Properly generates SHA256-based ETags (not timestamps)
2. **If-Match Headers** ✅ - Correctly checks If-Match for optimistic locking at HTTP layer
3. **PROPFIND Depth** ✅ - Handles Depth:0 and Depth:1 correctly
4. **Sync Tokens** ✅ - Atomic increment using database operations

### Missing Features ⚠️

Based on documentation vs. implementation:

1. **Recurrence Expansion** - Implemented but needs index on `(calendar_id, start)` for performance
2. **FOR UPDATE SKIP LOCKED** - Not found in worker queries (mentioned in README as required)
3. **Argon2id Password Hashing** - ✅ Implemented correctly in `caldav_auth.rs`

---

## Performance Concerns

### Database Indexes

Check if these indexes exist (not visible in code review):

```sql
-- Required for event queries:
CREATE INDEX idx_events_calendar_start ON events(calendar_id, start);
CREATE INDEX idx_events_calendar_uid ON events(calendar_id, uid);

-- Required for sync:
CREATE INDEX idx_calendars_sync_token ON calendars(sync_token);
```

### N+1 Query Risk

Review `db::events::list_events()` - ensure it doesn't trigger N+1 queries when fetching related data.

---

## Testing Gaps

### Missing Test Cases

1. **Concurrent Updates** - No test for the race condition in issue #3
2. **Version Mismatch** - No test verifying version check (issue #2)
3. **Type Safety** - No test catching ID type confusion (issue #1 would prevent this)

### Recommended Tests

```rust
#[tokio::test]
async fn test_concurrent_event_updates_with_version_check() {
    // Test that concurrent updates fail with version mismatch
}

#[tokio::test]
async fn test_concurrent_event_creation_no_duplicate() {
    // Test that concurrent creates don't cause duplicates
}
```

---

## Migration Requirements

If models change (e.g., adding newtypes), ensure:

1. ✅ Run `just db-new-migration <name>`
2. ✅ Update `cargo sqlx prepare` for offline query validation
3. ✅ Test migrations on production-like data volume

---

## Summary Table

| Issue | Severity | Location | Priority | Effort |
|-------|----------|----------|----------|--------|
| Raw Uuid instead of newtypes | 🔴 CRITICAL | core/models.rs | P0 | High |
| Missing version check in update | 🔴 CRITICAL | api/db/events.rs:238 | P0 | Medium |
| Race condition in PUT | 🟠 HIGH | api/routes/caldav.rs:154 | P1 | Medium |
| Dead code in bot | 🟡 MEDIUM | bot/db.rs | P2 | Low |
| Needless ? operators | 🟡 MEDIUM | bot/db.rs:489,518 | P2 | Low |

---

## Recommendations

### Immediate Actions (Before Production)

1. [ ] Fix critical issue #1: Implement newtypes for all IDs
2. [ ] Fix critical issue #2: Add version check in update_event
3. [ ] Add integration tests for concurrent scenarios
4. [ ] Run `cargo sqlx prepare` after changes
5. [ ] Verify database indexes exist

### Next Sprint

1. Fix high-priority issue #3: Race condition in PUT
2. Clean up dead code
3. Add missing integration tests
4. Performance testing with realistic data volumes

### Long-Term

1. Consider adding property-based tests for CalDAV compliance
2. Run `caldav-tester` tool mentioned in README
3. Add load testing for concurrent client scenarios

---

## Conclusion

The Televent codebase demonstrates good engineering practices with proper error handling, logging, and structure. However, the **3 critical issues identified must be resolved before production deployment** to prevent data loss, sync corruption, and security vulnerabilities.

The most important fix is implementing newtypes for IDs (issue #1), as this affects the entire codebase and prevents an entire class of type confusion bugs. Issues #2 and #3 are data integrity problems that will manifest under concurrent load.

Once these issues are addressed, the codebase will be production-ready with strong type safety and CalDAV compliance.

**Estimated Total Effort:** 2-3 weeks for all fixes and comprehensive testing.
