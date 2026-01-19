# Televent Architecture

This document provides visual architecture diagrams for the Televent calendar system.

## System Overview

```mermaid
graph TB
    subgraph "Clients"
        TG[Telegram App]
        WEB[Web Browser]
        CAL[Calendar Apps<br/>Apple/Google/Thunderbird]
    end

    subgraph "Televent System"
        subgraph "Frontend"
            DIOXUS[Dioxus Web App<br/>crates/web]
        end

        subgraph "Backend Services"
            API[Axum API Server<br/>crates/api]
            BOT[Telegram Bot<br/>crates/bot]
            WORKER[Background Worker<br/>crates/worker]
        end

        subgraph "Shared Libraries"
            CORE[Core Domain<br/>crates/core]
            MAILER[Email Service<br/>crates/mailer]
        end

        subgraph "Data Layer"
            PG[(PostgreSQL 16)]
        end
    end

    subgraph "External Services"
        TGAPI[Telegram Bot API]
        SMTP[SMTP Server]
    end

    TG --> TGAPI
    TGAPI --> BOT
    WEB --> DIOXUS
    DIOXUS --> API
    CAL -->|CalDAV| API

    API --> CORE
    API --> PG
    BOT --> CORE
    BOT --> PG
    WORKER --> CORE
    WORKER --> PG
    WORKER --> MAILER
    MAILER --> SMTP

    BOT --> TGAPI
```

## Crate Dependencies

```mermaid
graph BT
    subgraph "Binaries"
        API[api]
        BOT[bot]
        WORKER[worker]
        WEB[web]
    end

    subgraph "Libraries"
        CORE[televent-core]
        MAILER[mailer]
    end

    subgraph "External Crates"
        AXUM[axum]
        SQLX[sqlx]
        TELOXIDE[teloxide]
        DIOXUS[dioxus]
        TOKIO[tokio]
    end

    API --> CORE
    API --> AXUM
    API --> SQLX
    API --> TOKIO

    BOT --> CORE
    BOT --> TELOXIDE
    BOT --> SQLX
    BOT --> TOKIO

    WORKER --> CORE
    WORKER --> MAILER
    WORKER --> SQLX
    WORKER --> TOKIO

    WEB --> DIOXUS

    MAILER --> TOKIO
```

## API Request Flow

```mermaid
sequenceDiagram
    participant Client
    participant Router as Axum Router
    participant MW as Middleware
    participant Handler as Route Handler
    participant DB as Database Layer
    participant PG as PostgreSQL

    Client->>Router: HTTP Request
    Router->>MW: Route Matching

    alt CalDAV Request
        MW->>MW: Basic Auth Validation
        MW->>PG: Verify Device Password (Argon2id)
    else Web Request
        MW->>MW: Telegram InitData Validation
        MW->>MW: HMAC-SHA256 Verification
    end

    MW->>Handler: Authenticated Request
    Handler->>DB: Business Logic
    DB->>PG: SQL Query
    PG-->>DB: Result
    DB-->>Handler: Domain Model
    Handler-->>Client: HTTP Response
```

## CalDAV Protocol Flow

```mermaid
sequenceDiagram
    participant CAL as Calendar Client
    participant API as CalDAV Endpoint
    participant DB as Database

    Note over CAL,DB: Initial Sync
    CAL->>API: PROPFIND /caldav/{user_id}/<br/>Depth: 1
    API->>DB: Get calendar + events
    DB-->>API: Calendar metadata + event list
    API-->>CAL: 207 Multi-Status XML<br/>(hrefs, etags, ctag)

    Note over CAL,DB: Fetch Event
    CAL->>API: GET /caldav/{user_id}/{uid}.ics
    API->>DB: Get event by UID
    DB-->>API: Event data
    API-->>CAL: 200 OK<br/>text/calendar (iCal)

    Note over CAL,DB: Create/Update Event
    CAL->>API: PUT /caldav/{user_id}/{uid}.ics<br/>If-Match: "etag"
    API->>API: Validate ETag (optimistic lock)
    API->>DB: Create/Update event
    DB-->>API: New event + etag
    API->>DB: Increment sync_token
    API-->>CAL: 201 Created / 204 No Content<br/>ETag: "new-etag"

    Note over CAL,DB: Delete Event
    CAL->>API: DELETE /caldav/{user_id}/{uid}.ics<br/>If-Match: "etag"
    API->>DB: Delete event
    API->>DB: Increment sync_token
    API-->>CAL: 204 No Content
```

## CalDAV REPORT Method Flow

```mermaid
sequenceDiagram
    participant CAL as Calendar Client
    participant API as CalDAV Endpoint
    participant DB as Database

    Note over CAL,DB: Calendar Query (time-range filter)
    CAL->>API: REPORT /caldav/{user_id}/<br/>calendar-query XML
    API->>API: Parse time-range filter
    API->>DB: SELECT events WHERE start BETWEEN...
    DB-->>API: Matching events
    API->>API: Serialize to iCalendar
    API-->>CAL: 207 Multi-Status<br/>calendar-data for each event

    Note over CAL,DB: Sync Collection (incremental sync)
    CAL->>API: REPORT /caldav/{user_id}/<br/>sync-collection XML<br/>sync-token: "42"
    API->>API: Parse sync-token
    API->>DB: SELECT events WHERE sync_version > 42
    DB-->>API: Changed/new events
    API->>DB: Get current sync_token
    API-->>CAL: 207 Multi-Status<br/>changed events + new sync-token
```

## REST API Endpoints

```mermaid
flowchart LR
    subgraph "Events API"
        E1[POST /api/events]
        E2[GET /api/events]
        E3[GET /api/events/:id]
        E4[PUT /api/events/:id]
        E5[DELETE /api/events/:id]
    end

    subgraph "Device Passwords API"
        D1[POST /api/users/:user_id/devices]
        D2[GET /api/users/:user_id/devices]
        D3[DELETE /api/users/:user_id/devices/:device_id]
    end

    subgraph "Health API"
        H1[GET /health]
        H2[GET /health/ready]
    end

    subgraph "CalDAV Endpoints"
        C1[OPTIONS /caldav/]
        C2[PROPFIND /caldav/:user_id/]
        C3[REPORT /caldav/:user_id/]
        C4[GET /caldav/:user_id/:uid.ics]
        C5[PUT /caldav/:user_id/:uid.ics]
        C6[DELETE /caldav/:user_id/:uid.ics]
    end
```

## Device Password Management Flow

```mermaid
sequenceDiagram
    participant User
    participant API as REST API
    participant DB as Database
    participant CAL as Calendar Client

    Note over User,CAL: Create Device Password
    User->>API: POST /api/users/:id/devices<br/>{"name": "iPhone"}
    API->>API: Generate random password
    API->>API: Hash with Argon2id
    API->>DB: INSERT device_passwords
    DB-->>API: Device record
    API-->>User: 201 Created<br/>{"password": "abc123..."}

    Note over User,CAL: Use Device Password
    CAL->>API: CalDAV request<br/>Authorization: Basic base64(telegram_id:password)
    API->>API: Decode Basic Auth
    API->>DB: SELECT hashed_password WHERE user_id
    DB-->>API: Hash list
    API->>API: Argon2id.verify(password, hash)
    API->>DB: UPDATE last_used_at
    API-->>CAL: 200 OK (proceed with CalDAV)

    Note over User,CAL: Revoke Device Password
    User->>API: DELETE /api/users/:id/devices/:device_id
    API->>DB: DELETE device_passwords
    API-->>User: 204 No Content
```

## Database Schema

```mermaid
erDiagram
    users ||--o| calendars : "has one"
    users ||--o{ device_passwords : "has many"
    users ||--o| user_preferences : "has one"
    users ||--o{ audit_log : "has many"
    calendars ||--o{ events : "contains"

    users {
        uuid id PK
        bigint telegram_id UK
        string timezone
        timestamp created_at
    }

    calendars {
        uuid id PK
        uuid user_id FK,UK
        string name
        string color
        string sync_token
        string ctag
        timestamp created_at
        timestamp updated_at
    }

    events {
        uuid id PK
        uuid calendar_id FK
        string uid UK
        string summary
        string description
        string location
        timestamp start
        timestamp end
        boolean is_all_day
        enum status
        string rrule
        string timezone
        int version
        string etag
        timestamp created_at
        timestamp updated_at
    }

    device_passwords {
        uuid id PK
        uuid user_id FK
        string hashed_password
        string name
        timestamp last_used_at
        timestamp created_at
    }

    user_preferences {
        uuid user_id PK,FK
        int reminder_minutes_before
        time daily_digest_time
        boolean notifications_enabled
    }

    outbox_messages {
        uuid id PK
        enum status
        string message_type
        jsonb payload
        int retry_count
        timestamp scheduled_at
        timestamp processed_at
        timestamp created_at
    }

    audit_log {
        uuid id PK
        uuid user_id FK
        string action
        string entity_type
        uuid entity_id
        inet ip_address
        string user_agent
        timestamp created_at
    }
```

## Authentication Flows

### Web UI Authentication (Telegram Login Widget)

```mermaid
sequenceDiagram
    participant User
    participant Browser
    participant API
    participant Telegram

    User->>Browser: Click "Login with Telegram"
    Browser->>Telegram: Open Telegram OAuth
    Telegram->>User: Authorize request
    User->>Telegram: Approve
    Telegram->>Browser: Return initData<br/>(user, auth_date, hash)
    Browser->>API: Request with X-Telegram-Init-Data

    API->>API: Parse initData params
    API->>API: Sort params alphabetically
    API->>API: Build data-check-string
    API->>API: secret = HMAC-SHA256("WebAppData", bot_token)
    API->>API: computed_hash = HMAC-SHA256(secret, data-check-string)
    API->>API: Compare computed_hash with provided hash

    alt Valid Signature
        API->>API: Extract user_id from initData
        API->>API: Generate JWT (24h expiry)
        API-->>Browser: Set-Cookie: jwt=...
    else Invalid Signature
        API-->>Browser: 401 Unauthorized
    end
```

### CalDAV Authentication (HTTP Basic Auth)

```mermaid
sequenceDiagram
    participant Client as Calendar Client
    participant API as CalDAV Endpoint
    participant DB as PostgreSQL

    Client->>API: Request without auth
    API-->>Client: 401 WWW-Authenticate: Basic

    Client->>API: Authorization: Basic base64(telegram_id:password)
    API->>API: Decode base64
    API->>API: Split telegram_id:password

    API->>DB: SELECT id FROM users WHERE telegram_id = ?
    DB-->>API: user_id

    API->>DB: SELECT hashed_password FROM device_passwords WHERE user_id = ?
    DB-->>API: List of hashed passwords

    loop Each device password
        API->>API: Argon2id.verify(password, hash)
        alt Match found
            API->>DB: UPDATE device_passwords SET last_used_at = NOW()
            API->>API: Attach user_id to request
        end
    end

    alt Verified
        API-->>Client: 200 OK (with response)
    else Not Verified
        API-->>Client: 401 Unauthorized
    end
```

## Worker Message Processing (Outbox Pattern)

```mermaid
sequenceDiagram
    participant API as API Server
    participant DB as PostgreSQL
    participant Worker as Background Worker
    participant External as External Service<br/>(Email/Telegram)

    Note over API,External: Event Creation triggers notification

    API->>DB: INSERT event
    API->>DB: INSERT outbox_message<br/>(status=PENDING, payload=...)
    API-->>Client: 201 Created

    loop Poll every N seconds
        Worker->>DB: SELECT * FROM outbox_messages<br/>WHERE status = 'PENDING'<br/>FOR UPDATE SKIP LOCKED<br/>LIMIT 10

        DB-->>Worker: Messages to process

        loop Each message
            Worker->>DB: UPDATE status = 'PROCESSING'
            Worker->>External: Send notification

            alt Success
                Worker->>DB: UPDATE status = 'COMPLETED'
            else Failure
                Worker->>DB: UPDATE status = 'FAILED',<br/>retry_count++,<br/>scheduled_at = NOW() + backoff
            end
        end
    end
```

## ETag Generation

```mermaid
flowchart LR
    subgraph "Event Fields"
        UID[uid]
        SUM[summary]
        DESC[description]
        LOC[location]
        START[start]
        END[end]
        ALLDAY[is_all_day]
        STATUS[status]
        RRULE[rrule]
    end

    subgraph "Process"
        CONCAT[Concatenate with '|' separator]
        SHA[SHA-256 Hash]
        HEX[Hex Encode]
    end

    UID --> CONCAT
    SUM --> CONCAT
    DESC --> CONCAT
    LOC --> CONCAT
    START --> CONCAT
    END --> CONCAT
    ALLDAY --> CONCAT
    STATUS --> CONCAT
    RRULE --> CONCAT

    CONCAT --> SHA
    SHA --> HEX
    HEX --> ETAG[ETag]
```

## Directory Structure

```
televent/
├── crates/
│   ├── api/                 # Axum HTTP server
│   │   └── src/
│   │       ├── main.rs      # Server entry point
│   │       ├── config.rs    # Environment config
│   │       ├── error.rs     # Error types & conversions
│   │       ├── routes/      # HTTP handlers
│   │       │   ├── caldav.rs      # CalDAV protocol
│   │       │   ├── caldav_xml.rs  # XML generation
│   │       │   ├── events.rs      # REST API
│   │       │   ├── health.rs      # Health check
│   │       │   └── ical.rs        # iCal serialization
│   │       ├── middleware/  # Auth middleware
│   │       │   ├── caldav_auth.rs    # Basic Auth
│   │       │   └── telegram_auth.rs  # Telegram OAuth
│   │       └── db/          # Database layer
│   │           ├── calendars.rs
│   │           └── events.rs
│   │
│   ├── bot/                 # Telegram bot
│   │   └── src/main.rs
│   │
│   ├── core/                # Shared domain logic
│   │   └── src/
│   │       ├── models.rs    # Domain entities
│   │       ├── error.rs     # Domain errors
│   │       └── timezone.rs  # TZ utilities
│   │
│   ├── mailer/              # Email service
│   │   └── src/lib.rs
│   │
│   ├── web/                 # Dioxus frontend
│   │   └── src/main.rs
│   │
│   └── worker/              # Background jobs
│       └── src/main.rs
│
├── migrations/              # SQLx migrations
├── docs/                    # Documentation
├── Cargo.toml              # Workspace config
├── Justfile                # Task runner
├── docker-compose.yml      # Local services
└── CLAUDE.md               # AI assistant rules
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **ETag = SHA256(all fields)** | Clock-skew resistant. Avoids false conflicts from timestamp differences. |
| **Sync token = atomic counter** | `UPDATE ... RETURNING` ensures concurrent syncs never see same token. |
| **One calendar per user** | Simplifies CalDAV implementation. Enforced by unique index. |
| **Argon2id for passwords** | Memory-hard algorithm resistant to GPU attacks. |
| **Outbox pattern for notifications** | Ensures at-least-once delivery. Survives server crashes. |
| **`FOR UPDATE SKIP LOCKED`** | Prevents duplicate job processing in worker. |
| **No `unwrap()`/`expect()`** | Calendar data loss from panics is unacceptable. |
| **Newtypes for IDs** | Prevents mixing `UserId` with `CalendarId` at compile time. |

## Performance Indices

```sql
-- Fast event queries by calendar and time range
CREATE INDEX idx_events_calendar_start ON events(calendar_id, start);

-- Sync queries for modified events
CREATE INDEX idx_events_calendar_updated ON events(calendar_id, updated_at);

-- CalDAV UID lookups (unique per calendar)
CREATE UNIQUE INDEX idx_events_calendar_uid ON events(calendar_id, uid);

-- Worker polling for pending messages
CREATE INDEX idx_outbox_pending ON outbox_messages(status, scheduled_at)
    WHERE status = 'PENDING';

-- Audit log queries
CREATE INDEX idx_audit_user_time ON audit_log(user_id, created_at DESC);
```
