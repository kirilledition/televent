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
    WORKER --> TGAPI
    WORKER --> SMTP

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
    end

    subgraph "External Crates"
        AXUM[axum]
        SQLX[sqlx]
        TELOXIDE[teloxide]
        DIOXUS[dioxus]
        TOKIO[tokio]
        LETTRE[lettre]
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
    WORKER --> TELOXIDE
    WORKER --> LETTRE
    WORKER --> SQLX
    WORKER --> TOKIO

    WEB --> DIOXUS

    CORE --> SQLX
    CORE --> TOKIO
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
        MW->>MW: Basic Auth Validation (caldav_auth)
        MW->>PG: Verify Device Password (Argon2id)
    else API Request
        Note over MW: Currently passing IDs in path/query<br/>(Auth middleware planned)
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

## Database Schema

```mermaid
erDiagram
    users ||--o| calendars : "has one"
    users ||--o{ device_passwords : "has many"
    users ||--o| user_preferences : "has one"
    users ||--o{ audit_log : "has many"
    calendars ||--o{ events : "contains"
    events ||--o{ event_attendees : "has many"

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
        event_status status
        string rrule
        string timezone
        int version
        string etag
        timestamp created_at
        timestamp updated_at
    }

    event_attendees {
        uuid id PK
        uuid event_id FK
        string email
        bigint telegram_id
        attendee_role role
        participation_status status
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
        outbox_status status
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

## Worker Message Processing (Outbox Pattern)

```mermaid
sequenceDiagram
    participant API as API Server
    participant DB as PostgreSQL
    participant Worker as Background Worker
    participant TG as Telegram Bot API
    participant SMTP as SMTP Server

    Note over API,SMTP: Event Creation triggers notification

    API->>DB: INSERT event
    API->>DB: INSERT outbox_message<br/>(status=PENDING, payload=...)
    API-->>Client: 201 Created

    loop Poll every N seconds
        Worker->>DB: SELECT * FROM outbox_messages<br/>WHERE status = 'PENDING'<br/>FOR UPDATE SKIP LOCKED<br/>LIMIT 10

        DB-->>Worker: Messages to process

        loop Each message
            Worker->>DB: UPDATE status = 'PROCESSING'
            
            alt Message Type: telegram_notification
                Worker->>TG: Send message
            else Message Type: calendar_invite (The Interceptor)
                alt recipient is internal (@televent.internal)
                    Worker->>TG: Send Telegram Invite
                else recipient is external
                    Worker->>SMTP: Send Email (Planned)
                end
            end

            alt Success
                Worker->>DB: UPDATE status = 'COMPLETED'
            else Failure
                Worker->>DB: UPDATE status = 'FAILED',<br/>retry_count++,<br/>scheduled_at = NOW() + backoff
            end
        end
    end
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
│   │       │   ├── devices.rs     # Device passwords REST
│   │       │   ├── events.rs      # Events REST API
│   │       │   ├── health.rs      # Health check
│   │       │   └── ical.rs        # iCal serialization
│   │       ├── middleware/  # Auth middleware
│   │       │   ├── caldav_auth.rs    # Basic Auth
│   │       │   └── rate_limit.rs     # Rate limiting
│   │       └── db/          # Database layer (db-specific logic)
│   │           ├── calendars.rs
│   │           └── events.rs
│   │
│   ├── bot/                 # Telegram bot (Teloxide)
│   │   └── src/
│   │       ├── main.rs      # Bot entry point
│   │       ├── commands.rs  # Command definitions
│   │       ├── handlers.rs  # Update handlers
│   │       └── db.rs        # Bot-specific DB helpers
│   │
│   ├── core/                # Shared domain logic
│   │   └── src/
│   │       ├── lib.rs       # Crate entry
│   │       ├── models.rs    # Domain entities (Shared)
│   │       ├── error.rs     # Domain errors
│   │       ├── attendee.rs  # Attendee utilities (The Interceptor)
│   │       ├── recurrence.rs # Recurrence logic
│   │       └── timezone.rs  # TZ utilities
│   │
│   ├── web/                 # Dioxus frontend
│   │   └── src/main.rs
│   │
│   └── worker/              # Background jobs
│       └── src/
│           ├── main.rs      # Worker entry point
│           ├── processors.rs # Message processing logic
│           ├── mailer.rs    # Integrated email service
│           └── db.rs        # Worker-specific DB helpers
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
| **The Interceptor Pattern** | Internal invites (@televent.internal) are routed to Telegram, avoiding paid SMTP for MVP. |
| **ETag = SHA256(all fields)** | Clock-skew resistant. Avoids false conflicts from timestamp differences. |
| **Sync token = atomic counter** | `UPDATE ... RETURNING` ensures concurrent syncs never see same token. |
| **One calendar per user** | Simplifies CalDAV implementation. Enforced by unique index. |
| **Argon2id for passwords** | Memory-hard algorithm resistant to GPU attacks. |
| **Outbox pattern for notifications** | Ensures at-least-once delivery. Survives server crashes. |
| **`FOR UPDATE SKIP LOCKED`** | Prevents duplicate job processing in worker. |
| **No `unwrap()`/`expect()`** | Calendar data loss from panics is unacceptable. (Enforced by Clippy) |
| **Newtypes for IDs** | Prevents mixing `UserId` with `CalendarId` at compile time. |
