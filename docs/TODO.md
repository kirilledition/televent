# Televent TODO & Backlog

## Current Sprint

**Goal**: Implement CalDAV Server (Phase 3)
**Target**: Basic CalDAV sync working with Apple Calendar

### In Progress
- [ ] None

### Up Next
- [ ] CalDAV OPTIONS handler
- [ ] CalDAV PROPFIND implementation
- [ ] iCalendar serialization/deserialization

---

## Backlog

### High Priority (Core Functionality)

**Phase 3: CalDAV Server** 🎯
- [ ] Task 3.1: OPTIONS handler (capabilities discovery)
- [ ] Task 3.2: PROPFIND for calendar metadata (Depth: 0)
- [ ] Task 3.3: PROPFIND for event listing (Depth: 1)
- [ ] Task 3.4: REPORT calendar-query (time-range filtering)
- [ ] Task 3.5: REPORT sync-collection (delta sync)
- [ ] Task 3.6: GET single event (iCalendar serialization)
- [ ] Task 3.7: PUT create/update event (iCalendar parsing)
- [ ] Task 3.8: DELETE event with If-Match header
- [ ] Task 3.9: Recurrence expansion (RRULE → instances)
- [ ] Task 3.10: CalDAV compliance testing (caldav-tester)

**Phase 4: Telegram Bot**
- [ ] Task 4.1: Teloxide setup with /start command
- [ ] Task 4.2: Command routing (BotCommands derive)
- [ ] Task 4.3: /create event FSM (multi-step dialogue)
- [ ] Task 4.4: Natural language date parsing (chrono-english)
- [ ] Task 4.5: Event listing commands (/today, /tomorrow, /week)
- [ ] Task 4.6: Device password generation (/device add)

**Phase 5: Worker Process**
- [ ] Task 5.1: Outbox consumer with FOR UPDATE SKIP LOCKED
- [ ] Task 5.2: Email sender via Lettre
- [ ] Task 5.3: Telegram notification sender
- [ ] Task 5.4: Event reminder job (15min before)
- [ ] Task 5.5: Daily digest job (8am user timezone)

### Medium Priority (Polish & UX)

**Phase 2 Completion**
- [ ] Rate limiting middleware (tower-governor)
- [ ] Device passwords CRUD API
  - [ ] POST /api/device-passwords (generate)
  - [ ] GET /api/device-passwords (list)
  - [ ] DELETE /api/device-passwords/:id (revoke)
- [ ] Calendars CRUD API
  - [ ] POST /api/calendars
  - [ ] GET /api/calendars
  - [ ] PUT /api/calendars/:id
- [ ] Integration tests with test database

**Phase 6: Frontend (Dioxus)**
- [ ] Task 6.1: Dioxus project setup
- [ ] Task 6.2: Telegram Login Widget integration
- [ ] Task 6.3: Calendar view component (month/week/day)
- [ ] Task 6.4: Device password generator UI
- [ ] Task 6.5: Event creation form
- [ ] Task 6.6: Dark mode styling (Tailwind)

**Phase 7: GDPR Compliance**
- [ ] Task 7.1: Data export endpoint (GET /api/gdpr/export)
- [ ] Task 7.2: Account deletion endpoint (POST /api/gdpr/delete)
- [ ] Task 7.3: Permanent deletion worker (30-day cron)
- [ ] Task 7.4: Bot commands (/export, /delete_account)
- [ ] Privacy policy page
- [ ] Terms of service page

### Low Priority (Ops & Observability)

**Phase 8: Observability**
- [ ] Task 8.1: Prometheus metrics endpoint
  - [ ] http_requests_total counter
  - [ ] caldav_sync_duration histogram
  - [ ] outbox_queue_size gauge
  - [ ] bot_commands_total counter
- [ ] Task 8.2: OpenTelemetry tracing setup
- [ ] Task 8.3: Sentry error tracking integration
- [ ] Structured logging improvements
- [ ] Request ID tracing

**Phase 9: Deployment**
- [ ] Task 9.1: Fly.io configuration (fly.toml)
- [ ] Task 9.2: Multi-stage Dockerfile
- [ ] Task 9.3: GitHub Actions CI
  - [ ] Run tests
  - [ ] Check formatting
  - [ ] Run clippy
  - [ ] SQLx prepare --check
- [ ] Task 9.4: GitHub Actions CD (deploy on main)
- [ ] Task 9.5: Database backup strategy
- [ ] Task 9.6: Secrets management (fly secrets)

---

## Technical Debt

### Code Quality
- [ ] Clean up unused dynamic query builder in `db/events.rs` update_event()
- [ ] Wire middleware functions to actual routes
- [ ] Add `cargo sqlx prepare` to CI
- [ ] Implement newtype pattern (UserId, CalendarId, EventId)
- [ ] Add request ID middleware for tracing

### Testing
- [ ] Integration tests for events API (with test DB)
- [ ] Integration tests for auth middleware
- [ ] CalDAV protocol compliance tests
- [ ] Load testing (k6 or similar)
- [ ] Fuzzing for parsers (iCalendar, Telegram initData)

### Documentation
- [ ] API reference documentation (OpenAPI/Swagger)
- [ ] CalDAV client setup guides
  - [ ] Apple Calendar
  - [ ] Thunderbird
  - [ ] DAVx⁵ (Android)
- [ ] Bot commands help text
- [ ] Deployment guide
- [ ] Contributing guide

### Performance
- [ ] Database connection pooling tuning
- [ ] Add database query timeouts
- [ ] Consider caching layer (Redis) for frequently accessed data
- [ ] Profile and optimize hot paths
- [ ] Add database indexes based on production query patterns

### Security
- [ ] Security audit of auth mechanisms
- [ ] Implement request size limits
- [ ] Add CORS configuration for web UI
- [ ] JWT token refresh mechanism
- [ ] Brute force protection for auth endpoints
- [ ] SQL injection prevention audit

---

## Ideas / Future Features

### Nice to Have
- [ ] Multiple calendars per user (lift one-calendar constraint)
- [ ] Calendar sharing (read-only, read-write permissions)
- [ ] Calendar subscriptions (read-only URLs)
- [ ] Event attachments (within size limit)
- [ ] Event attendees (basic support)
- [ ] Recurring event exceptions (EXDATE)
- [ ] WebSocket support for real-time updates
- [ ] Dark/light theme toggle
- [ ] Calendar color customization
- [ ] Event categories/tags
- [ ] Search functionality
- [ ] Export to ICS file
- [ ] Import from ICS file

### Experimental
- [ ] AI-powered natural language event creation
- [ ] Smart reminders based on location
- [ ] Integration with other calendar services (Google, Outlook)
- [ ] Mobile app (React Native or Flutter)
- [ ] Desktop app (Tauri)
- [ ] Browser extension for quick event creation

---

## Blocked Items

None currently.

---

## Recently Completed

### 2026-01-18
- ✅ Phase 0: Project Setup (6db0d7b)
- ✅ Phase 1: Core Domain (53ab92d)
- ✅ Phase 2 Core: Backend API with auth (76b72f9)
- ✅ Phase 2 CRUD: Events API with full tests (1120f69)
- ✅ Comprehensive development documentation
- ✅ TODO tracking system

---

## Notes

### Decision Log
- **2026-01-18**: Chose Axum over Actix-web for better type safety
- **2026-01-18**: Chose Argon2id over bcrypt for password hashing (more secure)
- **2026-01-18**: Decided to implement CalDAV before bot (can test sync immediately)

### Questions / Discussions
- Should we add WebSocket support for real-time calendar updates? (Would require architectural changes)
- Do we need a separate read-replica for heavy read workloads? (Premature optimization?)
- Should we implement GraphQL instead of REST? (REST is simpler for MVP)

---

*Last Updated: 2026-01-18*
