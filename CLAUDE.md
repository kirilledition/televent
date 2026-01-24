<televent_rules>

<critical_rules>
## Critical Rules (Production System - Zero Tolerance)

- **NO unwrap()/expect()**: Use `?` or explicit error handling. Why: We handle calendars - data loss from panics is unacceptable.
- **NO println!**: Use `tracing::info!` or `tracing::error!`. Why: We need structured logs for debugging production CalDAV sync issues.
- **Type Safety**: Use newtypes (`UserId(Uuid)`, not raw `Uuid`). Why: CalDAV spec has many ID types - type confusion causes sync corruption.
- **Async Runtime**: Always `tokio`. Why: Entire stack (Axum, SQLx, Teloxide) is tokio-based.
</critical_rules>

<tech_stack>
## Tech Stack

- **Backend**: Axum 0.7 + SQLx (Postgres 16) + Teloxide (Telegram bot)
- **Frontend**: Dioxus + Tailwind CSS (dark mode first: bg-zinc-950)
- **Tooling**: Rust 1.75+, Just (command runner), Docker Compose
</tech_stack>

<project_structure>
## Project Structure (Monorepo)

- `crates/core`: Domain logic (pure Rust, no I/O)
- `crates/api`: Axum server (CalDAV + REST)
- `crates/bot`: Telegram bot (Event creation implemented)
- `crates/worker`: Outbox consumer
- `crates/web`: Dioxus frontend
- `migrations/`: SQLx migrations
</project_structure>

<operational_commands>
## Common Commands (Use Just)

- `just setup`: Start Docker, run migrations
- `just test`: Run all tests
- `just dev-api`: Hot-reload API server
- `just dev-bot`: Hot-reload bot
- `just db-reset`: Drop/recreate database
</operational_commands>

<database_workflow>
## Database Workflow (SQLx)

- **ALWAYS create migration** after changing `core/src/models.rs`.
- `sqlx migrate add <descriptive_name>`
- `cargo sqlx prepare`: Generate offline query metadata.
</database_workflow>

<caldav_implementation>
## CalDAV Implementation Rules

- **ETag = SHA256(event serialized)**: Never use timestamps.
- **Sync token**: Must increment atomically (Postgres sequence or `UPDATE ... RETURNING`).
- **PROPFIND Depth**: Depth:0 = metadata, Depth:1 = children.
- **Recurrence expansion**: Expand RRULE in queries, not at event creation.
</caldav_implementation>

<testing_requirements>
## Testing Requirements

- **Unit tests**: All `core` functions.
- **Integration tests**: Each API endpoint + CalDAV method.
- **Compliance tests**: Run `caldav-tester` before merging CalDAV changes.
</testing_requirements>

<error_handling>
## Error Handling Patterns

- **Libraries**: Use `thiserror` (typed errors).
- **Binaries**: Use `anyhow` (context propagation).
</error_handling>

<current_state>
## Current Implementation State

- **Bot Event Creation**: Fully implemented (parsing multi-line text).
- **CalDAV Auth**: Implemented with WWW-Authenticate header.
- **CalDAV Properties**: getlastmodified and proper XML namespaces implemented.
- **Known Issue**: Investigating GUI client sync issues (Thunderbird).
</current_state>

</televent_rules>