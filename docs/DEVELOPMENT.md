# Televent Development Log

## Project Overview
Telegram-native calendar management system with CalDAV sync support. Built with Rust, Axum, PostgreSQL, and Dioxus.

**Repository**: kirilledition/televent
**Branch**: claude/setup-project-K4qu0
**Started**: 2026-01-18

---

## Completed Phases

### Phase 0: Project Setup ✅ (Commit: 6db0d7b)

**What We Built:**
- Cargo workspace with 6 crates (core, api, bot, worker, mailer, web)
- Docker Compose (PostgreSQL 16, Mailpit, Jaeger)
- Justfile with development commands
- Complete documentation structure
- Configuration files (.env.example, Dioxus.toml, tailwind.config.js)

**Key Decisions:**
- Monorepo structure for easier dependency management
- Renamed `core` crate to `televent-core` (avoiding stdlib conflict)
- Manual Dioxus setup (CLI not available)
- SQLx for compile-time query verification

**Lessons Learned:**
- `cargo new` creates git repos in subdirectories - must remove before committing
- Dioxus 0.5 API changed significantly (no more `cx: Scope`, use `#[component]` instead)
- Lettre requires explicit feature flags to avoid conflicts (use `tokio1-rustls-tls`)

---

### Phase 1: Core Domain ✅ (Commit: 53ab92d)

**What We Built:**
- 9 PostgreSQL migrations with full schema
- Timezone handling module with chrono-tz
- 13 unit tests for models and timezone conversion

**Database Schema Highlights:**
- `event_status` and `outbox_status` custom enums
- Automatic `updated_at` triggers on calendars/events/user_preferences
- Audit triggers for GDPR compliance
- Optimistic locking with `version` field
- SHA256-based ETags for conflict detection
- Partial indexes for performance (e.g., `outbox_messages.status = 'pending'`)

**Key Decisions:**
- One calendar per user (hard constraint via unique index)
- ETag = SHA256(serialized event) NOT timestamp (avoids clock skew issues)
- 30-day soft delete grace period for GDPR compliance
- Audit logs retained for 2 years (legal requirement)

**Lessons Learned:**
- SQLx derive requires direct `#[derive(sqlx::FromRow)]` not `cfg_attr`
- Chrono `Timelike` trait must be in scope for `.hour()`, `.minute()` methods
- Doc tests need all imports in example code

**Testing Strategy:**
- Unit tests for all business logic
- Timezone conversion tests (UTC ↔ Singapore as reference)
- Serialization tests for all models

---

### Phase 2: Backend API ✅ (Commits: 76b72f9, 1120f69)

**What We Built:**

#### Core Infrastructure (76b72f9)
- Axum server with health check endpoint
- Environment-based configuration
- Error handling with `ApiError` enum
- Telegram auth middleware (HMAC-SHA256 validation)
- Structured logging with tracing

#### Full CRUD API (1120f69)
- CalDAV Basic Auth middleware (Argon2id verification)
- Events database repository layer
- Complete REST API for events (Create, Read, Update, Delete)
- 35 passing unit tests

**API Endpoints:**
```
GET  /health              - Health check with DB connectivity
POST /api/events          - Create event (201 Created)
GET  /api/events          - List events with filters (calendar_id, start, end)
GET  /api/events/:id      - Get single event
PUT  /api/events/:id      - Update event (partial updates)
DELETE /api/events/:id    - Delete event (204 No Content)
```

**Key Decisions:**

1. **Two Authentication Mechanisms:**
   - Telegram Auth: HMAC-SHA256 signature via `X-Telegram-Init-Data` header
   - CalDAV Auth: HTTP Basic Auth with `telegram_id:password` format

2. **Error Handling Architecture:**
   - `ApiError` for HTTP responses (NotFound, BadRequest, Unauthorized, etc.)
   - Automatic conversion from `CalendarError` → `ApiError`
   - SQL constraint violations mapped to 409 Conflict

3. **Database Layer Pattern:**
   - Repository pattern in `db/` module
   - All queries use SQLx with compile-time verification
   - COALESCE for partial updates in PUT requests

4. **ETag Generation:**
   - SHA256 hash of: `uid|summary|start|end`
   - Regenerated on every update
   - Used for conflict detection (future CalDAV implementation)

**Lessons Learned:**

1. **Base64 Decoding:**
   - Use `base64::engine::general_purpose::STANDARD`
   - New API changed from `decode()` to `engine.decode()`

2. **Password Hashing:**
   - Argon2 `verify_password()` returns `Result<(), Error>`
   - Must check `.is_ok()` not compare boolean directly

3. **Axum Middleware:**
   - Middleware functions take `Request`, `Next`, return `Response`
   - Use `request.extensions_mut().insert()` to pass data to handlers
   - Must use `State<T>` extractor for shared state

4. **SQLx Partial Updates:**
   - COALESCE allows optional updates: `SET field = COALESCE($1, field)`
   - Bind `Option<T>` directly - SQLx handles NULL correctly

5. **Ownership Issues:**
   - `.unwrap_or()` moves value - use `.clone().unwrap_or()` when value needed later
   - Or use `.as_ref().unwrap_or()` if you only need reference

**Testing Strategy:**
- Unit tests for every function with side effects
- Test happy path AND all error cases
- Password hashing tested with actual Argon2id
- HTTP Basic Auth parsing tested with edge cases (colons in password, invalid base64, etc.)

**Code Quality Metrics:**
- 35 passing tests (0 failures)
- Zero `unwrap()` or `expect()` in production code
- Zero `println!` - all logging via tracing
- All warnings addressed

---

## Design Patterns Used

### 1. Repository Pattern
```rust
// Separates data access from business logic
pub async fn create_event(pool: &PgPool, ...) -> Result<Event, ApiError>
```

### 2. Error Conversion Pattern
```rust
impl From<CalendarError> for ApiError {
    fn from(err: CalendarError) -> Self { ... }
}
// Allows using ? operator across error types
```

### 3. Middleware Pattern
```rust
pub async fn caldav_basic_auth(
    State(pool): State<PgPool>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError>
```

### 4. Newtype Pattern (Planned)
```rust
// From CLAUDE.md - not yet implemented
pub struct UserId(Uuid);
pub struct CalendarId(Uuid);
```

---

## What DIDN'T Work (Anti-patterns Discovered)

### 1. ❌ Using `cfg_attr` for SQLx Derives
**Problem:**
```rust
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
```
Caused warnings about unexpected cfg values.

**Solution:**
```rust
#[derive(sqlx::FromRow)]  // Direct derive
```

### 2. ❌ Importing tower::ServiceExt Without Feature
**Problem:** Tower's ServiceExt requires `util` feature flag.

**Solution:** Use Axum's re-export or don't use in tests (we removed it).

### 3. ❌ Moving Values in `.unwrap_or()`
**Problem:**
```rust
let new_summary = summary.unwrap_or(current.summary);  // Moves summary
// ... later ...
.bind(summary)  // Error: value used after move
```

**Solution:**
```rust
let new_summary = summary.clone().unwrap_or(current.summary);
```

### 4. ❌ Forgetting to Import Traits
**Problem:** `.hour()` and `.minute()` methods not found on DateTime.

**Solution:** Import `chrono::Timelike` trait.

---

## Dependencies Rationale

### Core Dependencies
- **tokio**: Async runtime (required by Axum, SQLx, Teloxide)
- **axum 0.7**: Web framework (type-safe, fast, good ecosystem)
- **sqlx 0.7**: Compile-time verified SQL queries
- **chrono + chrono-tz**: Timezone-aware date/time handling

### Security
- **argon2**: Password hashing (memory-hard, recommended for passwords)
- **hmac + sha2**: HMAC-SHA256 for Telegram signature validation

### Utilities
- **hex**: Converting hash bytes to hex strings
- **urlencoding**: Decoding Telegram initData
- **base64**: HTTP Basic Auth parsing

### Why NOT某些 Libraries
- **bcrypt**: Argon2id is more modern and configurable
- **actix-web**: Axum has better type safety and smaller binary
- **diesel**: SQLx provides compile-time query checking without macros

---

## Critical Code Paths

### 1. Telegram Authentication Flow
```
1. Client sends X-Telegram-Init-Data header
2. Parse initData into key-value pairs
3. Extract hash, sort remaining params
4. Compute HMAC-SHA256(secret_key, data-check-string)
5. Compare hashes in constant time
6. Extract user_id from initData
7. Insert user_id into request extensions
```

### 2. CalDAV Authentication Flow
```
1. Client sends Authorization: Basic base64(telegram_id:password)
2. Decode base64, split on first colon
3. Look up user by telegram_id
4. Fetch all device_passwords for user
5. Try verifying password against each hash (Argon2id)
6. Update last_used_at timestamp
7. Insert user_id into request extensions
```

### 3. Event Update with Optimistic Locking
```
1. Fetch current event (includes version)
2. Apply partial updates with COALESCE
3. Increment version
4. Regenerate ETag
5. Update updated_at timestamp
6. Return updated event or 404
```

---

## Database Insights

### Performance Indexes
```sql
-- Time-based queries (most common)
CREATE INDEX idx_events_calendar_start ON events(calendar_id, start);

-- CalDAV sync-collection
CREATE INDEX idx_events_calendar_updated ON events(calendar_id, updated_at);

-- Worker efficiency (partial index)
CREATE INDEX idx_outbox_pending ON outbox_messages(status, scheduled_at)
    WHERE status = 'pending';
```

### Trigger Functions
```sql
-- Auto-update updated_at
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Audit logging
CREATE OR REPLACE FUNCTION log_event_change()
RETURNS TRIGGER AS $$
-- Automatically logs to audit_log table
```

### Why These Choices
- Partial index on `outbox_messages.status = 'pending'`: Only pending messages need fast lookup
- Composite index `(calendar_id, start)`: Most queries filter by calendar then sort by time
- Triggers for `updated_at`: Ensures consistency, can't forget to update

---

## Testing Philosophy

### What We Test
1. **Every function with side effects** (database writes, external calls)
2. **All error paths** (not just happy path)
3. **Edge cases** (empty strings, colons in passwords, invalid base64, etc.)
4. **Data transformations** (serialization, ETag generation, etc.)

### What We DON'T Test
1. Third-party library internals (trust Axum, SQLx, etc.)
2. Trivial getters/setters
3. Type definitions without logic

### Test Organization
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_path() { ... }

    #[test]
    fn test_error_case_1() { ... }

    #[test]
    fn test_edge_case() { ... }
}
```

### Integration Tests (Future)
```
tests/
  integration/
    events_api_test.rs   // Full HTTP request/response cycle
    caldav_sync_test.rs  // CalDAV protocol compliance
```

---

## Environment Variables Reference

```bash
# Required
DATABASE_URL=postgresql://user:pass@localhost:5432/televent
JWT_SECRET=random_256_bit_string
TELEGRAM_BOT_TOKEN=123456:ABC-DEF...

# Optional (with defaults)
API_HOST=0.0.0.0
API_PORT=3000
RUST_LOG=info,api=debug,sqlx=warn
```

---

## Common Commands

```bash
# Development
just setup           # Start Docker, run migrations, build
just dev-api         # Hot-reload API server
just test            # Run all tests
just db-reset        # Drop and recreate database

# Testing
cargo test -p televent-core    # Core domain tests only
cargo test -p api              # API tests only
cargo test --workspace         # All tests

# Database
just db-console               # Open psql shell
just db-create-migration name # Create new migration
cargo sqlx prepare            # Generate offline query metadata

# Quality
just lint            # Check formatting and clippy
just fmt             # Auto-format code
```

---

## Current Status (2026-01-18)

### ✅ Complete
- Phase 0: Project Setup
- Phase 1: Core Domain (models, migrations, timezone handling)
- Phase 2: Backend API (auth, CRUD, 35 passing tests)

### 🚧 In Progress
- Nothing currently

### 📋 TODO (From Master Plan)

**Phase 2 Remaining:**
- [ ] Rate limiting middleware (Task 2.4) - optional but recommended
- [ ] REST API for device passwords (Task 2.5)
- [ ] REST API for calendars

**Phase 3: CalDAV Server**
- [ ] OPTIONS handler (capabilities discovery)
- [ ] PROPFIND (calendar metadata, event listing)
- [ ] REPORT (calendar-query, sync-collection)
- [ ] GET single event (iCalendar format)
- [ ] PUT create/update event (parse iCalendar)
- [ ] DELETE event
- [ ] Recurrence expansion (RRULE handling)
- [ ] CalDAV compliance testing

**Phase 4: Telegram Bot**
- [ ] Teloxide setup
- [ ] Command routing (/start, /today, /create, etc.)
- [ ] Event creation FSM (Finite State Machine)
- [ ] Natural language date parsing (chrono-english)
- [ ] Event listing commands
- [ ] Device password generation commands

**Phase 5: Worker Process**
- [ ] Outbox consumer loop
- [ ] Email sender (Lettre)
- [ ] Telegram notification sender
- [ ] Event reminder jobs
- [ ] Daily digest jobs

**Phase 6: Frontend (Dioxus)**
- [ ] Dioxus setup
- [ ] Telegram Login Widget
- [ ] Calendar view component
- [ ] Device password generator UI

**Phase 7: GDPR Compliance**
- [ ] Data export endpoint
- [ ] Account deletion endpoint with 30-day grace
- [ ] Permanent deletion worker
- [ ] Bot integration for /export and /delete_account

**Phase 8: Observability**
- [ ] Prometheus metrics
- [ ] OpenTelemetry tracing
- [ ] Sentry integration

**Phase 9: Deployment**
- [ ] Fly.io configuration
- [ ] Multi-stage Dockerfile
- [ ] GitHub Actions CI/CD
- [ ] Database backups
- [ ] Secrets management

---

## Known Issues / Technical Debt

1. **SQLx Offline Mode Not Set Up**
   - Need to run `cargo sqlx prepare` with database running
   - CI will fail without offline query metadata
   - **Fix**: Run `just setup` then `cargo sqlx prepare`

2. **Middleware Functions Unused**
   - `validate_telegram_init_data()` not wired into routes yet
   - `caldav_basic_auth()` not wired into CalDAV routes yet
   - **Fix**: Add to CalDAV routes when implementing Phase 3

3. **No Rate Limiting Yet**
   - API endpoints are unprotected from abuse
   - **Fix**: Implement tower-governor middleware (Phase 2 Task 2.4)

4. **Event Update Query Has Unused Variables**
   - `query`, `params`, `param_count` built but not used
   - Using simpler COALESCE approach instead
   - **Fix**: Clean up unused code or implement dynamic query builder

5. **Missing Integration Tests**
   - Only unit tests so far
   - No end-to-end HTTP request tests
   - **Fix**: Add `tests/integration/` directory with actual HTTP tests

---

## Future Considerations

### Scalability
- Consider connection pooling limits (default: max_connections = 10)
- Add caching layer for frequently accessed calendars
- Database read replicas for heavy read workloads

### Security
- Implement rate limiting per user/IP
- Add request ID tracing for security audit
- Consider adding CORS properly for web UI
- Implement JWT token refresh mechanism

### Features
- WebSocket support for real-time calendar updates
- Calendar sharing (read-only, read-write)
- Multiple calendars per user (lift current constraint)
- Recurring event exceptions (EXDATE handling)

### DevOps
- Health check should verify all services (DB, Redis if added, etc.)
- Add /metrics endpoint for Prometheus
- Add /debug endpoints for development
- Database migration rollback testing

---

## References

- [RFC 4791 - CalDAV](https://datatracker.ietf.org/doc/html/rfc4791)
- [RFC 5545 - iCalendar](https://datatracker.ietf.org/doc/html/rfc5545)
- [RFC 6578 - Sync Collection](https://datatracker.ietf.org/doc/html/rfc6578)
- [Telegram Bot API](https://core.telegram.org/bots/api)
- [Axum Documentation](https://docs.rs/axum/)
- [SQLx Documentation](https://docs.rs/sqlx/)

---

## Team Notes

**Code Review Checklist:**
- [ ] No `unwrap()` or `expect()` (use `?` or explicit error handling)
- [ ] No `println!` (use `tracing::info!`, `tracing::error!`)
- [ ] All public functions have doc comments
- [ ] Tests cover happy path + error cases
- [ ] Migration includes both UP and rollback logic
- [ ] New dependencies justified in this document

**When Adding New Features:**
1. Update this document with design decisions
2. Add tests BEFORE implementation (TDD preferred)
3. Update README.md if user-facing
4. Run `just lint` before committing
5. Update Phase completion checklist

---

*Last Updated: 2026-01-18*
*Next Review: After Phase 3 completion*
