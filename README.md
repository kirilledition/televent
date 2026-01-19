# Televent: Master Plan

## 1. Vision & Constraints

**Goal**: Telegram-native calendar management with CalDAV sync (no Google/Microsoft account required)

**Hard Constraints**:
- One calendar per user
- No shared calendars
- No attachments (text-only events)
- GDPR compliant (EU users)

**Tech Stack**: Rust, Axum, Dioxus, PostgreSQL, Teloxide, Docker

---

## 2. Architecture

```
/televent
├── .devcontainer/
├── .github/workflows/
│   ├── ci.yml
│   └── deploy.yml
├── Cargo.toml          # Workspace root
├── Justfile
├── Dioxus.toml
├── tailwind.config.js
├── docker-compose.yml
├── fly.toml            # Deployment config
├── crates/
│   ├── core/           # Domain logic (pure Rust)
│   ├── api/            # Axum (CalDAV + REST API)
│   ├── bot/            # Teloxide bot
│   ├── worker/         # Outbox consumer
│   ├── web/            # Dioxus frontend
│   └── mailer/         # Email sender
├── migrations/         # SQLx migrations
└── docs/
    ├── bot_commands.md
    ├── caldav_compliance.md
    └── gdpr_procedures.md
```

---

## 3. Data Model

### **Core Entities**

```rust
// crates/core/src/models.rs

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub telegram_id: i64,
    pub telegram_username: Option<String>,
    pub created_at: DateTime<Utc>,
    pub timezone: String,  // IANA timezone (e.g., "Asia/Singapore")
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct Calendar {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: String,           // Hex color for UI
    pub sync_token: String,      // RFC 6578 sync token
    pub ctag: String,            // Collection tag for change detection
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub uid: String,             // iCalendar UID (stable across syncs)
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub is_all_day: bool,
    pub status: EventStatus,     // CONFIRMED | TENTATIVE | CANCELLED
    pub rrule: Option<String>,   // RFC 5545 recurrence rule
    pub timezone: String,        // VTIMEZONE reference
    pub version: i32,            // Optimistic locking
    pub etag: String,            // HTTP ETag for conflict detection
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "event_status", rename_all = "UPPERCASE")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

#[derive(sqlx::FromRow)]
pub struct DevicePassword {
    pub id: Uuid,
    pub user_id: Uuid,
    pub hashed_password: String,  // Argon2id hash
    pub name: String,              // User-friendly label (e.g., "iPhone")
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
pub struct OutboxMessage {
    pub id: Uuid,
    pub message_type: String,    // "email" | "telegram_notification"
    pub payload: sqlx::types::Json<serde_json::Value>,
    pub status: OutboxStatus,
    pub retry_count: i32,
    pub scheduled_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "outbox_status", rename_all = "lowercase")]
pub enum OutboxStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub action: String,          // "event_created" | "data_exported" | "account_deleted"
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

### **Database Indexes**

```sql
-- migrations/003_indexes.sql
CREATE INDEX idx_events_calendar_start ON events(calendar_id, start);
CREATE INDEX idx_events_uid ON events(uid);
CREATE INDEX idx_events_updated_at ON events(calendar_id, updated_at);
CREATE INDEX idx_outbox_pending ON outbox_messages(status, scheduled_at) 
    WHERE status = 'pending';
CREATE INDEX idx_audit_user_time ON audit_log(user_id, created_at DESC);
```

---

## 4. Authentication & Authorization

### **Two Auth Mechanisms**

**1. Telegram Authentication (Web UI + Bot)**
- Use Telegram Login Widget (webapp.telegram.com)
- Validate `initData` HMAC-SHA256 with bot token
- Session: JWT stored in httpOnly cookie (24h expiry)

**2. Device Password (CalDAV Clients)**
- User generates password via bot: `/device add iPhone`
- CalDAV client uses HTTP Basic Auth: `username=telegram_id:password`
- Store Argon2id hash in `device_passwords` table

### **Auth Flow Diagram**

```
Telegram User → Bot /start → Create User record → Generate device password
                ↓
          Web UI (Telegram OAuth) → JWT cookie → Axum middleware
                ↓
    CalDAV Client (HTTP Basic) → Validate device password → Attach User context
```

---

## 5. CalDAV Implementation

### **Required Methods**

| Method   | Endpoint                            | Purpose                                            |
| -------- | ----------------------------------- | -------------------------------------------------- |
| OPTIONS  | `/caldav/`                          | Capabilities discovery                             |
| PROPFIND | `/caldav/{user_id}/`                | Calendar metadata                                  |
| PROPFIND | `/caldav/{user_id}/calendar.ics`    | Event listing                                      |
| REPORT   | `/caldav/{user_id}/`                | Filtered queries (calendar-query, sync-collection) |
| GET      | `/caldav/{user_id}/{event_uid}.ics` | Fetch single event                                 |
| PUT      | `/caldav/{user_id}/{event_uid}.ics` | Create/update event                                |
| DELETE   | `/caldav/{user_id}/{event_uid}.ics` | Delete event                                       |

### **Key Features**

- **Sync Token Strategy**: Increment `calendar.sync_token` on any change
- **ETag/CTag**: 
  - ETag = `event.etag` (SHA256 of serialized event)
  - CTag = `calendar.ctag` (timestamp of last calendar change)
- **Conflict Handling**: PUT with `If-Match: <etag>` → 412 Precondition Failed if mismatch
- **Recurrence**: Parse RRULE with `rrule` crate, expand instances in queries

### **Compliance Testing**

Use Apple's `caldav-tester` suite:
```bash
git clone https://github.com/apple/ccs-caldavtester
python testcaldav.py --server localhost:3000 --all
```

---

## 6. Bot Commands & Flows

**Core Commands** (`docs/bot_commands.md`):

```
/start - Initialize account, show setup instructions
/today - List today's events
/tomorrow - List tomorrow's events
/week - List this week's events
/create - Create event (guided conversation)
/cancel <event_id> - Cancel event
/device add <name> - Generate CalDAV password
/device list - Show all device passwords
/device revoke <id> - Delete device password
/export - Request GDPR data export
/delete_account - Initiate account deletion
```

**Event Creation Flow**:
```
User: /create
Bot: "Event title?"
User: "Team standup"
Bot: "When? (e.g., 'tomorrow 10am' or '2026-01-20 14:00')"
User: "tomorrow 10am"
Bot: "Duration? (e.g., '30m', '1h')"
User: "30m"
Bot: "Description? (or /skip)"
User: "Weekly sync meeting"
Bot: ✅ "Event created: Team standup, Jan 19 10:00-10:30"
```

**Natural Language Parsing**: Use `chrono-english` for date parsing

### **Notifications**

- Reminder 15min before event start
- Daily digest at 8am (user's timezone)
- Store preferences in `user_preferences` table:
  ```sql
  CREATE TABLE user_preferences (
      user_id UUID PRIMARY KEY,
      reminder_minutes_before INT DEFAULT 15,
      daily_digest_time TIME DEFAULT '08:00:00',
      notifications_enabled BOOL DEFAULT true
  );
  ```

---

## 7. Rate Limiting

**Library**: `tower-governor` (rate limiting middleware for Axum)

**Strategy**:
- **CalDAV endpoints**: 100 requests/minute per user
- **REST API**: 300 requests/minute per user
- **Telegram Bot**: Use Teloxide's built-in throttle (20 msg/sec)

**Implementation**:
```rust
// crates/api/src/middleware/rate_limit.rs
use tower_governor::{Governor, GovernorConfigBuilder};

pub fn caldav_rate_limiter() -> Governor {
    let config = Box::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(100)
            .finish()
            .unwrap()
    );
    Governor::new(&config)
}
```

---

## 8. Worker Process (Outbox Consumer)

**Responsibilities**:
- Poll `outbox_messages` table for `status = 'pending'`
- Send emails via Lettre
- Send Telegram notifications via Teloxide
- Retry failed jobs with exponential backoff

**Algorithm**:
```rust
// crates/worker/src/main.rs
loop {
    let jobs = sqlx::query_as::<_, OutboxMessage>(
        "UPDATE outbox_messages 
         SET status = 'processing' 
         WHERE id IN (
             SELECT id FROM outbox_messages 
             WHERE status = 'pending' AND scheduled_at <= NOW()
             ORDER BY scheduled_at 
             LIMIT 10 
             FOR UPDATE SKIP LOCKED
         )
         RETURNING *"
    ).fetch_all(&pool).await?;

    for job in jobs {
        match process_job(&job).await {
            Ok(_) => mark_completed(&job.id).await?,
            Err(e) if job.retry_count < 5 => {
                let backoff = 2_u64.pow(job.retry_count as u32) * 60; // 1m, 2m, 4m, 8m, 16m
                reschedule(&job.id, backoff).await?;
            }
            Err(_) => mark_failed(&job.id).await?,
        }
    }

    tokio::time::sleep(Duration::from_secs(10)).await;
}
```

---

## 9. GDPR Compliance

### **Required Capabilities**

**1. Data Export** (`/export` command):
- Generate JSON file with all user data
- Include: events, calendars, device passwords (hashed), audit log
- Send via Telegram file upload
- Log export request in `audit_log`

**2. Account Deletion** (`/delete_account` command):
- Two-step confirmation (prevent accidents)
- Cascade delete: user → calendars → events → device_passwords → audit_log
- Send confirmation email
- 30-day grace period before permanent deletion

**3. Data Retention**:
- Audit logs: 2 years (legal requirement)
- Outbox messages: 90 days (then archive)
- Deleted users: 30 days in `deleted_users` table (soft delete)

**Implementation**: `docs/gdpr_procedures.md`

```sql
-- Soft delete
CREATE TABLE deleted_users (
    id UUID PRIMARY KEY,
    telegram_id BIGINT,
    deletion_requested_at TIMESTAMP,
    permanent_deletion_at TIMESTAMP,
    data_snapshot JSONB  -- Encrypted backup for recovery
);

-- Audit every operation
CREATE OR REPLACE FUNCTION log_event_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO audit_log (user_id, action, entity_type, entity_id)
    VALUES (
        NEW.user_id, 
        TG_OP || '_event',
        'event',
        NEW.id
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

---

## 10. Observability

**Metrics** (Prometheus + Grafana):
- `http_requests_total{endpoint, status}`
- `caldav_sync_duration_seconds{method}`
- `outbox_queue_size{status}`
- `bot_commands_total{command}`

**Tracing** (OpenTelemetry → Jaeger):
- Distributed tracing for CalDAV sync operations
- Span attributes: `user_id`, `calendar_id`, `event_count`

**Error Tracking**: Sentry (capture panics + anyhow errors)

**Setup**:
```rust
// crates/api/src/observability.rs
use axum::middleware;
use opentelemetry_otlp::WithExportConfig;

pub fn init_telemetry() {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://jaeger:4317")
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();

    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();
}
```

---

## 11. Implementation Roadmap

### Progress Overview

| Phase | Status | Progress |
|-------|--------|----------|
| Phase 0: Project Setup | ✅ Complete | 100% |
| Phase 1: Core Domain | ✅ Complete | 100% |
| Phase 2: Backend API | 🔄 In Progress | 85% |
| Phase 3: CalDAV Server | 🔄 In Progress | 85% |
| Phase 4: Telegram Bot | 🔄 In Progress | 75% |
| Phase 5: Worker Process | ⏳ Pending | 0% |
| Phase 6: Frontend (Dioxus) | ⏳ Pending | 0% |
| Phase 7: GDPR Compliance | ⏳ Pending | 0% |
| Phase 8: Observability | ⏳ Pending | 0% |
| Phase 9: Deployment | ⏳ Pending | 0% |

**Recent Updates (2026-01-19):**
- ✅ Updated all dependencies to latest versions (tokio 1.49, axum 0.8, sqlx 0.8, teloxide 0.13, etc.)
- 🔄 Added rate limiting structure (placeholder, full implementation pending)
- 🔄 Added RRULE validation (basic implementation, expansion pending)
- ✅ Implemented Telegram bot with 11 commands (/start, /today, /tomorrow, /week, etc.)
- ✅ Bot database integration with event querying
- ✅ Fixed breaking changes from dependency updates

---

### **Phase 0: Project Setup** ✅

**Task 0.1**: Initialize workspace ✅
```bash
cargo new --lib crates/core
cargo new --bin crates/api
cargo new --bin crates/bot
cargo new --bin crates/worker
cargo new --lib crates/mailer
dx new crates/web
```

**Validation**: `cargo build --workspace` succeeds ✅

**Task 0.2**: Docker Compose ✅
```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: televent
      POSTGRES_PASSWORD: dev
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
  
  mailpit:
    image: axllent/mailpit
    ports:
      - "1025:1025"
      - "8025:8025"
  
  jaeger:
    image: jaegertracing/all-in-one
    ports:
      - "16686:16686"  # UI
      - "4317:4317"    # OTLP gRPC

volumes:
  pgdata:
```

**Validation**: `docker-compose up -d && docker ps` shows 3 running containers ✅

**Task 0.3**: Justfile commands ✅
```makefile
# Justfile
setup:
    docker-compose up -d
    cargo sqlx migrate run
    cargo build --workspace

test:
    cargo test --workspace

lint:
    cargo fmt --check
    cargo clippy --workspace -- -D warnings

db-reset:
    cargo sqlx database drop -y
    cargo sqlx database create
    cargo sqlx migrate run

dev-api:
    cargo watch -x 'run -p api'

dev-bot:
    cargo watch -x 'run -p bot'

dev-web:
    dx serve --hot-reload
```

**Validation**: `just setup && just test` passes ✅

---

### **Phase 1: Core Domain** ✅

**Task 1.1**: Define models ✅
- Create `crates/core/src/models.rs` with structs from Section 3
- Add derives: `FromRow`, `Serialize`, `Deserialize`
- Create custom types: `UserId(Uuid)`, `CalendarId(Uuid)`, `EventId(Uuid)`

**Validation**: `cargo test -p core` (unit test serialization) ✅

**Task 1.2**: Database migrations ✅
```bash
sqlx migrate add create_users
sqlx migrate add create_calendars
sqlx migrate add create_events
sqlx migrate add create_device_passwords
sqlx migrate add create_outbox
sqlx migrate add create_audit_log
sqlx migrate add add_indexes
```

**Validation**: `cargo sqlx prepare --check` succeeds ✅

**Task 1.3**: Error types ✅
```rust
// crates/core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CalendarError {
    #[error("Event not found: {0}")]
    EventNotFound(Uuid),
    
    #[error("Version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: i32, actual: i32 },
    
    #[error("Invalid recurrence rule: {0}")]
    InvalidRRule(String),
}
```

**Validation**: Error conversion tests ✅

**Task 1.4**: Timezone handling ✅
- Add `chrono-tz` dependency
- Create `parse_user_timezone(tz: &str) -> Result<Tz>`
- Store `timezone` in User table
- Use in event creation/display

**Validation**: Unit test UTC ↔ Singapore conversion ✅

---

### **Phase 2: Backend API** 🔄

**Task 2.1**: Axum setup ✅
```rust
// crates/api/src/main.rs
#[tokio::main]
async fn main() {
    let pool = PgPool::connect(&env::var("DATABASE_URL")?).await?;

    let app = Router::new()
        .route("/health", get(health_check))
        .layer(Extension(pool));

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;
}
```

**Validation**: `curl localhost:3000/health` returns 200 ✅

**Task 2.2**: Telegram auth middleware ✅
```rust
// crates/api/src/middleware/telegram_auth.rs
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub async fn validate_telegram_init_data(
    State(bot_token): State<String>,
    headers: HeaderMap,
    mut req: Request<Body>,
) -> Result<Request<Body>, StatusCode> {
    let init_data = headers
        .get("X-Telegram-Init-Data")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let is_valid = verify_telegram_hash(init_data, &bot_token);
    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Parse user_id from init_data, attach to request extensions
    Ok(req)
}
```

**Validation**: Unit test with valid/invalid HMAC ✅

**Task 2.3**: Device password auth ✅
```rust
// crates/api/src/middleware/caldav_auth.rs
pub async fn caldav_basic_auth(
    State(pool): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    mut req: Request<Body>,
) -> Result<Request<Body>, StatusCode> {
    let telegram_id: i64 = auth.username().parse()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let device = sqlx::query_as::<_, DevicePassword>(
        "SELECT * FROM device_passwords WHERE user_id = (
            SELECT id FROM users WHERE telegram_id = $1
        )"
    )
    .bind(telegram_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let is_valid = argon2::verify_encoded(
        &device.hashed_password,
        auth.password().as_bytes()
    ).unwrap_or(false);
    
    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    req.extensions_mut().insert(device.user_id);
    Ok(req)
}
```

**Validation**: Integration test with valid/invalid credentials ✅

**Task 2.4**: Rate limiting 🔄 (Placeholder)
- Created `middleware/rate_limit.rs` structure
- Documented target rates (100 req/min CalDAV, 300 req/min REST)
- TODO: Complete tower_governor integration with correct generic parameters

**Validation**: Deferred pending full implementation

**Task 2.5**: REST API endpoints ✅
- `POST /api/events` - Create event ✅
- `GET /api/events?start=<ts>&end=<ts>` - List events ✅
- `PUT /api/events/:id` - Update event ✅
- `DELETE /api/events/:id` - Delete event ✅
- `POST /api/users/:user_id/devices` - Generate device password ✅
- `GET /api/users/:user_id/devices` - List device passwords ✅
- `DELETE /api/users/:user_id/devices/:device_id` - Revoke device password ✅

**Validation**: Integration tests for each endpoint ✅

---

### **Phase 3: CalDAV Server** 🔄

**Task 3.1**: OPTIONS handler ✅
```rust
async fn caldav_options() -> impl IntoResponse {
    (
        [
            ("DAV", "1, calendar-access"),
            ("Allow", "OPTIONS, PROPFIND, REPORT, GET, PUT, DELETE"),
        ],
        StatusCode::OK,
    )
}
```

**Validation**: `curl -X OPTIONS localhost:3000/caldav/` returns DAV header ✅

**Task 3.2**: PROPFIND (calendar metadata) ✅
```rust
// Parse XML body, check Depth header (0 or 1)
// Return calendar properties: displayname, ctag, supported-calendar-component-set
```

**Validation**: Test with `curl -X PROPFIND -H "Depth: 0"` ✅

**Task 3.3**: PROPFIND (event listing) ✅
```rust
// Return list of event hrefs with etags
```

**Validation**: Python caldav client fetches event list ✅

**Task 3.4**: REPORT calendar-query ✅
```rust
// Parse <calendar-query> XML with <time-range> filter
// Return matching events as iCalendar data
```

**Validation**: Query events between two dates ✅

**Task 3.5**: REPORT sync-collection ✅
```rust
// Client sends sync-token, return changes since token
// Increment sync_token on calendar after changes
```

**Validation**: Create event, sync, verify delta response ✅

**Task 3.6**: GET single event ✅
```rust
// Serialize Event to iCalendar format with icalendar crate
```

**Validation**: Fetch event, parse with Python icalendar library ✅

**Task 3.7**: PUT create/update ✅
```rust
// Parse iCalendar from request body
// Check If-Match header for optimistic locking
// Update event.version, event.etag, calendar.ctag
```

**Validation**: Conflict test (two clients update same event) ✅

**Task 3.8**: DELETE ✅
```rust
// Check If-Match header
// Soft delete (set status = CANCELLED) or hard delete
```

**Validation**: Delete event, verify 404 on GET ✅

**Task 3.9**: Recurrence expansion 🔄 (Basic validation)
- Added `core/src/recurrence.rs` with RRULE validation
- `validate_rrule()` checks for required FREQ parameter
- Placeholder functions for future expansion
- TODO: Complete rrule crate integration for full event expansion

**Validation**: Basic RRULE validation tests passing ✅

**Task 3.10**: Full compliance test ⏳
```bash
just test-caldav  # Runs caldav-tester suite
```

**Validation**: 100% pass rate on caldav-tester

---

### **Phase 4: Telegram Bot** 🔄 (75% Complete)

**Task 4.1**: Teloxide setup ✅
- Implemented complete bot infrastructure with `Command::repl()`
- Database integration with SQLx (runtime query validation)
- Configuration from environment variables
- Structured logging with tracing
- Created modules: main.rs, commands.rs, handlers.rs, db.rs, config.rs

**Validation**: Bot compiles and runs successfully ✅

**Task 4.2**: Command routing ✅
Implemented 11 commands with BotCommands derive:
- `/start` - Welcome message with quick start guide
- `/help` - Comprehensive command help
- `/today` - Show today's events with time & location
- `/tomorrow` - Show tomorrow's events
- `/week` - Show next 7 days of events
- `/create` - Event creation guide (interactive flow pending)
- `/list` - Event listing options
- `/cancel` - Event cancellation guide
- `/device` - CalDAV device management info
- `/export` - Calendar export (placeholder)
- `/deleteaccount` - GDPR deletion info

**Validation**: All commands route to correct handlers ✅

**Task 4.3**: /create flow (FSM)
```rust
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};

#[derive(Clone, Default)]
enum CreateEventState {
    #[default]
    Start,
    AwaitingTitle,
    AwaitingTime { title: String },
    AwaitingDuration { title: String, start: DateTime<Utc> },
    AwaitingDescription { title: String, start: DateTime<Utc>, duration: Duration },
}
```

**Validation**: Complete event creation flow in Telegram

**Task 4.4**: Natural language date parsing
```rust
use chrono_english::{parse_date_string, Dialect};

fn parse_user_date(input: &str, user_tz: Tz) -> Result<DateTime<Utc>> {
    let parsed = parse_date_string(input, user_tz.to_utc(), Dialect::Uk)?;
    Ok(parsed.with_timezone(&Utc))
}
```

**Validation**: Test "tomorrow 3pm", "next monday 10:30", "2026-01-25 14:00"

**Task 4.5**: Event listing
```rust
async fn list_events(
    bot: Bot,
    msg: Message,
    pool: PgPool,
    range: DateRange,
) -> ResponseResult<()> {
    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE start >= $1 AND start < $2"
    )
    .bind(range.start)
    .bind(range.end)
    .fetch_all(&pool)
    .await?;
    
    let text = events.iter()
        .map(|e| format!("• {} - {}", e.start.format("%H:%M"), e.summary))
        .collect::<Vec<_>>()
        .join("\n");
    
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}
```

**Validation**: `/today` returns today's events

**Task 4.6**: Device password generation
```rust
async fn device_add(name: String, user_id: Uuid, pool: PgPool) -> Result<String> {
    let password = generate_password(16); // Random alphanumeric
    let hash = argon2::hash_encoded(
        password.as_bytes(),
        b"salt",  // Use proper random salt
        &argon2::Config::default()
    )?;
    
    sqlx::query(
        "INSERT INTO device_passwords (user_id, name, hashed_password) VALUES ($1, $2, $3)"
    )
    .bind(user_id)
    .bind(name)
    .bind(hash)
    .execute(&pool)
    .await?;
    
    Ok(password)
}
```

**Validation**: `/device add iPhone` returns password, stored in DB

---

### **Phase 5: Worker Process**

**Task 5.1**: Outbox consumer loop
```rust
// See Section 8 implementation
```

**Validation**: Insert test job, verify processed within 15 seconds

**Task 5.2**: Email sender
```rust
// crates/mailer/src/lib.rs
use lettre::{Message, SmtpTransport, Transport};

pub async fn send_email(to: &str, subject: &str, body: &str) -> Result<()> {
    let email = Message::builder()
        .from("noreply@televent.app".parse()?)
        .to(to.parse()?)
        .subject(subject)
        .body(body.to_string())?;
    
    let mailer = SmtpTransport::relay("mailpit")?
        .port(1025)
        .build();
    
    mailer.send(&email)?;
    Ok(())
}
```

**Validation**: Check Mailpit UI (localhost:8025) for sent email

**Task 5.3**: Telegram notification sender
```rust
pub async fn send_telegram_notification(
    bot: Bot,
    telegram_id: i64,
    message: &str,
) -> Result<()> {
    bot.send_message(ChatId(telegram_id), message).await?;
    Ok(())
}
```

**Validation**: Receive notification in Telegram

**Task 5.4**: Event reminder job
```rust
// On event creation, insert outbox job:
sqlx::query(
    "INSERT INTO outbox_messages (message_type, payload, scheduled_at)
     VALUES ('telegram_notification', $1, $2)"
)
.bind(json!({
    "telegram_id": user.telegram_id,
    "message": format!("Reminder: {} starts in 15 minutes", event.summary)
}))
.bind(event.start - Duration::minutes(15))
.execute(&pool)
.await?;
```

**Validation**: Create event, wait for reminder

**Task 5.5**: Daily digest job
```rust
// Cron job (or in-app scheduler) inserts daily jobs at midnight
```

**Validation**: Receive digest at 8am

---

### **Phase 6: Frontend (Dioxus Web)**

**Task 6.1**: Dioxus setup
```rust
// crates/web/src/main.rs
use dioxus::prelude::*;

fn main() {
    dioxus_web::launch(App);
}

fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-50",
            h1 { "Televent" }
        }
    })
}
```

**Validation**: `dx build && dx serve` loads page

**Task 6.2**: Telegram Login Widget
```html
<script async src="https://telegram.org/js/telegram-widget.js?22"
    data-telegram-login="YourBotUsername"
    data-size="large"
    data-auth-url="https://televent.app/auth/telegram"
    data-request-access="write">
</script>
```

**Validation**: Click widget, redirected to /auth/telegram with initData

**Task 6.3**: Calendar view component
```rust
#[component]
fn CalendarView(cx: Scope) -> Element {
    let events = use_future(cx, (), |_| async {
        fetch_events().await.unwrap()
    });
    
    cx.render(rsx! {
        div { class: "grid grid-cols-7 gap-2",
            for event in events.get().unwrap_or(&vec![]) {
                EventCard { event: event.clone() }
            }
        }
    })
}
```

**Validation**: View displays events from API

**Task 6.4**: Device password generator UI
```rust
#[component]
fn DevicePasswordGenerator(cx: Scope) -> Element {
    let password = use_state(cx, || None);
    
    let generate = move |_| {
        cx.spawn(async move {
            let pwd = api_generate_password().await.unwrap();
            password.set(Some(pwd));
        });
    };
    
    cx.render(rsx! {
        button { onclick: generate, "Generate Password" }
        if let Some(pwd) = password.get() {
            p { "Password: {pwd}" }
        }
    })
}
```

**Validation**: Button generates password, displays on screen

---

### **Phase 7: GDPR Compliance**

**Task 7.1**: Data export endpoint
```rust
// crates/api/src/routes/gdpr.rs
async fn export_user_data(
    Extension(user_id): Extension<Uuid>,
    State(pool): State<PgPool>,
) -> Result<Json<UserDataExport>> {
    let user = get_user(&user_id, &pool).await?;
    let events = get_all_events(&user_id, &pool).await?;
    let devices = get_device_passwords(&user_id, &pool).await?;
    let audit = get_audit_log(&user_id, &pool).await?;
    
    // Log export request
    log_audit(&pool, &user_id, "data_exported").await?;
    
    Ok(Json(UserDataExport {
        user,
        events,
        devices,
        audit_log: audit,
        exported_at: Utc::now(),
    }))
}
```

**Validation**: `/export` returns complete JSON, logged in audit_log

**Task 7.2**: Account deletion endpoint
```rust
async fn delete_account(
    Extension(user_id): Extension<Uuid>,
    State(pool): State<PgPool>,
    Json(confirmation): Json<DeleteConfirmation>,
) -> Result<StatusCode> {
    if !confirmation.confirmed {
        return Err(BadRequest("Must confirm deletion"));
    }
    
    // Snapshot data
    let snapshot = export_user_data_internal(&user_id, &pool).await?;
    
    // Move to deleted_users table
    sqlx::query(
        "INSERT INTO deleted_users (id, telegram_id, data_snapshot, deletion_requested_at, permanent_deletion_at)
         SELECT id, telegram_id, $1, NOW(), NOW() + INTERVAL '30 days'
         FROM users WHERE id = $2"
    )
    .bind(serde_json::to_value(snapshot)?)
    .bind(user_id)
    .execute(&pool)
    .await?;
    
    // Cascade delete
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await?;
    
    Ok(StatusCode::NO_CONTENT)
}
```

**Validation**: Delete account, verify 30-day grace period

**Task 7.3**: Permanent deletion worker
```rust
// Runs daily, checks deleted_users.permanent_deletion_at
async fn purge_expired_accounts(pool: &PgPool) {
    sqlx::query("DELETE FROM deleted_users WHERE permanent_deletion_at < NOW()")
        .execute(pool)
        .await?;
}
```

**Validation**: Mock expired deletion, verify purge

**Task 7.4**: Bot integration
```rust
// /export command
async fn handle_export(bot: Bot, msg: Message, pool: PgPool) {
    let user_id = get_user_id_from_telegram(&msg.from().unwrap().id, &pool).await?;
    let data = export_user_data_internal(&user_id, &pool).await?;
    
    let json = serde_json::to_string_pretty(&data)?;
    let file = InputFile::memory(json.as_bytes()).file_name("televent_data.json");
    
    bot.send_document(msg.chat.id, file).await?;
}

// /delete_account command (two-step confirmation)
```

**Validation**: Receive JSON file via Telegram, confirm deletion flow

---

### **Phase 8: Observability**

**Task 8.1**: Prometheus metrics
```rust
// crates/api/src/metrics.rs
use prometheus::{IntCounterVec, HistogramVec, register_int_counter_vec, register_histogram_vec};

lazy_static! {
    static ref HTTP_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["endpoint", "status"]
    ).unwrap();
    
    static ref CALDAV_DURATION: HistogramVec = register_histogram_vec!(
        "caldav_sync_duration_seconds",
        "CalDAV sync duration",
        &["method"]
    ).unwrap();
}

// Middleware to record metrics
```

**Validation**: `curl localhost:3000/metrics` returns Prometheus format

**Task 8.2**: OpenTelemetry tracing
```rust
// See Section 10 implementation
```

**Validation**: View traces in Jaeger UI (localhost:16686)

**Task 8.3**: Sentry integration
```rust
// crates/api/src/main.rs
sentry::init(("https://YOUR_SENTRY_DSN", sentry::ClientOptions {
    release: Some(env!("CARGO_PKG_VERSION").into()),
    ..Default::default()
}));

// In error handlers
sentry::capture_error(&error);
```

**Validation**: Trigger error, verify Sentry dashboard

---

### **Phase 9: Deployment**

**Task 9.1**: Fly.io configuration
```toml
# fly.toml
app = "televent"
primary_region = "sin"  # Singapore

[build]
  dockerfile = "Dockerfile"

[env]
  DATABASE_URL = "postgres://..."
  RUST_LOG = "info"

[[services]]
  internal_port = 3000
  protocol = "tcp"

  [[services.ports]]
    handlers = ["http"]
    port = 80
  
  [[services.ports]]
    handlers = ["tls", "http"]
    port = 443

[mounts]
  source = "televent_data"
  destination = "/data"
```

**Validation**: `fly deploy` succeeds

**Task 9.2**: Multi-container Dockerfile
```dockerfile
# Build stage
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/api /usr/local/bin/
COPY --from=builder /app/target/release/bot /usr/local/bin/
COPY --from=builder /app/target/release/worker /usr/local/bin/

# Supervisor to run multiple processes
COPY supervisord.conf /etc/supervisor/conf.d/
CMD ["/usr/bin/supervisord"]
```

**Validation**: Build image, run locally

**Task 9.3**: GitHub Actions CI/CD
```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace
      - run: cargo clippy -- -D warnings
      - run: cargo sqlx prepare --check
```

```yaml
# .github/workflows/deploy.yml
name: Deploy
on:
  push:
    branches: [main]
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: superfly/flyctl-actions/setup-flyctl@master
      - run: flyctl deploy --remote-only
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```

**Validation**: Push to main triggers deployment

**Task 9.4**: Database backups
```bash
# Fly.io Postgres automated backups (configure in Fly dashboard)
# Or custom backup script:
pg_dump $DATABASE_URL | gzip > backup_$(date +%Y%m%d).sql.gz
# Upload to S3 or equivalent
```

**Validation**: Restore from backup successfully

**Task 9.5**: Secrets management
```bash
fly secrets set TELEGRAM_BOT_TOKEN=xxx
fly secrets set DATABASE_URL=postgres://...
fly secrets set SENTRY_DSN=https://...
```

**Validation**: App starts with correct env vars

---

## 12. Pre-Commit Hooks

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: cargo-fmt
        name: Cargo Format
        entry: cargo fmt -- --check
        language: system
        pass_filenames: false
      
      - id: cargo-clippy
        name: Cargo Clippy
        entry: cargo clippy --workspace -- -D warnings
        language: system
        pass_filenames: false
      
      - id: cargo-test
        name: Cargo Test
        entry: cargo test --workspace
        language: system
        pass_filenames: false
      
      - id: sqlx-check
        name: SQLx Check
        entry: cargo sqlx prepare --check
        language: system
        pass_filenames: false
      
      - id: dx-build
        name: Dioxus Build
        entry: dx build --release
        language: system
        pass_filenames: false
```

**Install**: `pre-commit install`

---

## 13. Testing Strategy

### **Unit Tests**
- All `crates/core` functions (100% coverage goal)
- Date parsing edge cases
- iCalendar serialization/deserialization

### **Integration Tests**
```rust
// tests/integration/caldav_sync.rs
#[tokio::test]
async fn test_full_sync_flow() {
    let pool = setup_test_db().await;
    let client = caldav_client();
    
    // Create event via API
    let event = create_test_event(&pool).await;
    
    // Sync via CalDAV
    let synced = client.sync().await.unwrap();
    assert_eq!(synced[0].uid, event.uid);
    
    // Modify via CalDAV
    client.update_event(&event.uid, "New title").await.unwrap();
    
    // Verify in DB
    let updated = get_event(&pool, &event.id).await.unwrap();
    assert_eq!(updated.summary, "New title");
}
```

### **End-to-End Tests**
```python
# tests/e2e/test_telegram_flow.py
def test_create_event_via_bot():
    bot = TelegramClient()
    bot.send_command("/create")
    bot.send_message("Test event")
    bot.send_message("tomorrow 10am")
    bot.send_message("1h")
    
    # Verify event in CalDAV
    caldav = CalDAVClient()
    events = caldav.get_events()
    assert any(e.summary == "Test event" for e in events)
```

### **Performance Tests**
- CalDAV sync with 1000 events (target: <2s)
- Concurrent PUT requests (100 clients)
- Worker throughput (1000 jobs/min)

---

## 14. Documentation Structure

```
docs/
├── api_reference.md          # REST API endpoints
├── caldav_reference.md       # CalDAV endpoints + examples
├── bot_commands.md           # Telegram bot commands
├── caldav_compliance.md      # Compliance test results
├── gdpr_procedures.md        # Data export/deletion flows
├── deployment_guide.md       # Fly.io setup
└── architecture_diagrams/
    ├── auth_flow.png
    ├── caldav_sync.png
    └── data_model.png
```

---

## 15. Success Criteria

**Phase 1-4 Complete**:
- [ ] User can create account via Telegram
- [ ] User can create events via bot
- [ ] CalDAV client can sync events
- [ ] No `unwrap()` in codebase

**Phase 5-7 Complete**:
- [ ] Reminders sent 15min before events
- [ ] Data export works via `/export`
- [ ] Account deletion with 30-day grace period

**Phase 8-9 Complete**:
- [ ] Prometheus metrics exposed
- [ ] Deployed to Fly.io with auto-scaling
- [ ] 100% CalDAV compliance (caldav-tester)

**Production Ready**:
- [ ] 99.9% uptime over 30 days
- [ ] <100ms p99 latency for API
- [ ] Zero data loss incidents
- [ ] GDPR audit passed