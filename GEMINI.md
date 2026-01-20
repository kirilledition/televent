## Critical Rules (Production System - Zero Tolerance)

**NO unwrap()/expect()**: Use `?` or explicit error handling. Why: We handle calendars - data loss from panics is unacceptable.

**NO println!**: Use `tracing::info!` or `tracing::error!`. Why: We need structured logs for debugging production CalDAV sync issues.

**Type Safety**: Use newtypes (`UserId(Uuid)`, not raw `Uuid`). Why: CalDAV spec has many ID types - type confusion causes sync corruption.

**Async Runtime**: Always `tokio`. Why: Entire stack (Axum, SQLx, Teloxide) is tokio-based.

## Tech Stack

- **Backend**: Axum 0.7 + SQLx (Postgres 16) + Teloxide (Telegram bot)
- **Frontend**: Dioxus + Tailwind CSS (dark mode first: bg-zinc-950)
- **Tooling**: Rust 1.75+, Just (command runner), Docker Compose

## Project Structure (Monorepo)
```
televent/
├── crates/
│   ├── core/      # Domain logic (pure Rust, no I/O)
│   ├── api/       # Axum server (CalDAV + REST)
│   ├── bot/       # Telegram bot
│   ├── worker/    # Outbox consumer
│   └── web/       # Dioxus frontend
├── migrations/    # SQLx migrations
└── docs/          # Architecture diagrams, API specs
```

## Dependency Management

**CRITICAL: Use `cargo add`, NEVER edit Cargo.toml manually**

```bash
# Add a dependency to a specific crate
cargo add --package bot chrono-english

# Add with workspace features
cargo add --package api tower-http --features trace,cors

# Update all dependencies to latest compatible versions
cargo update

# Update to latest versions (including breaking changes)
cargo upgrade  # Requires: cargo install cargo-edit
```

Why: Manual edits miss version resolution, feature flags, and workspace inheritance.

## Common Commands (Use Just)
```bash
just setup          # Start Docker, run migrations
just test           # Run all tests
just dev-api        # Hot-reload API server
just dev-bot        # Hot-reload bot
just db-reset       # Drop/recreate database
```

## Database Workflow (SQLx)

**ALWAYS create migration after changing `core/src/models.rs`**:
```bash
sqlx migrate add <descriptive_name>
# Edit migration SQL
cargo sqlx prepare  # Generate offline query metadata
```

Why: SQLx validates queries at compile time - missing migrations break the build.

## CalDAV Implementation Rules

**ETag = SHA256(event serialized)**: Never use timestamps. Why: Clock skew between client/server causes false conflicts.

**Sync token must increment atomically**: Use Postgres sequence or `UPDATE ... RETURNING`. Why: Concurrent syncs must never see same token.

**PROPFIND Depth header matters**: Depth:0 = metadata only, Depth:1 = + children. Why: Apple Calendar always sends Depth:1 - must return event list.

**Recurrence expansion**: Expand RRULE in queries, not at event creation. Why: Storage efficiency - one DB row for infinite instances.

## Error Handling Patterns
```rust
// Libraries (core, mailer): use thiserror
#[derive(Error, Debug)]
pub enum CalendarError {
    #[error("Event not found: {0}")]
    EventNotFound(Uuid),
}

// Binaries (api, bot, worker): use anyhow
async fn handle_request() -> anyhow::Result<Response> {
    // ... context with .context("meaningful message")?
}
```

Why: Libraries need typed errors for consumers. Binaries need quick error propagation.

## Testing Requirements

**Unit tests**: All `core` functions (models, business logic)
**Integration tests**: Each API endpoint + CalDAV method
**Compliance tests**: Run `caldav-tester` before merging CalDAV changes

Coverage threshold: 80% for core, 60% for API handlers.

## Authentication Flow

- **Web UI**: Telegram Login Widget → HMAC-SHA256 validation → JWT cookie (24h)
- **CalDAV**: HTTP Basic Auth (telegram_id:device_password) → Argon2id verification

Why two mechanisms: Telegram OAuth is seamless for users. CalDAV clients need stable credentials.

## Code Style

- **Format**: `cargo fmt` (default Rust style)
- **Linting**: `cargo clippy -- -D warnings` (all warnings are errors)
- **Imports**: Group std, external crates, internal crates (rustfmt handles this)

## Common Mistakes to Avoid

❌ Modifying events without incrementing `version` field → CalDAV conflicts
❌ Returning UTC timestamps without timezone info → Client display bugs  
❌ Using `.to_string()` on errors → Lose error context, debugging nightmare
❌ Querying events without index on `(calendar_id, start)` → Table scans at scale
❌ Forgetting `FOR UPDATE SKIP LOCKED` in worker → Duplicate job processing

## When Stuck

1. Check `migrations/` - does schema match `core/src/models.rs`?
2. Run `cargo sqlx prepare --check` - are queries valid?
3. Read `docs/caldav_compliance.md` - are we following RFC 5545?
4. Check Docker logs - is Postgres/Mailpit running?

## External References

- CalDAV spec: RFC 4791 (calendar-access), RFC 6578 (sync-collection)
- iCalendar format: RFC 5545
- Telegram Bot API: https://core.telegram.org/bots/api