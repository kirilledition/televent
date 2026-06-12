<televent_rules>

<critical_rules>

- **NO unwrap()/expect()**: Use `?` or explicit error handling. Why: We handle calendars - data loss from panics is unacceptable.
- **NO println!**: Use `tracing::info!` or `tracing::error!`. Why: We need structured logs for debugging production CalDAV sync issues.
- **Type Safety**: Use domain newtypes (`UserId(i64)`, event `Uuid`, iCalendar UID `String`) instead of interchangeable primitives. Why: CalDAV has many ID types and type confusion causes sync corruption.
- **Async Runtime**: Always `tokio`. Why: Entire stack (Axum, SQLx, Teloxide) is tokio-based.
</critical_rules>

<guidelines>

- **Testing**: Write tests for all code you write and aim for at least 80% coverage.
- **Documentation**: Update documentation concurrently with any architectural changes.
</guidelines>

<tech_stack>
- **Backend**: Axum + Supabase + Teloxide
- **Tooling**: Rust, Nix, Just, Cargo
- **Frontend**: TypeScript + Next.js + Tailwind CSS (catppuccin) + tma.js (telegram) + pnpm (package manager)
</tech_stack>

<operational_commands>

- use all tools through `nix develop`, nothing is installed directly on the system
- never add or remove any dependencies, always use `cargo add` or `cargo remove`

- `just lint`: Check backend quality without mutating files (check, fmt --check, clippy)
- `just lint-frontend`: Check frontend quality (eslint)
- `just typecheck-frontend`: Check frontend TypeScript types
- `just fmt-frontend`: Format frontend code (prettier)
- `just build-docker`: Build unified Railway Docker image
- `just run`: Run app
- `just test`: Run fast non-DB backend tests and doc tests
- `just test-db`: Run the full backend suite, including DB-backed `sqlx::test` cases; requires `DATABASE_URL`
- `just test-coverage`: Run tests with coverage report
- `just db-reset`: Reset database (Supabase reset + migrations)
- `just db-start`: Start Supabase services
- `just db-status`: Check Supabase status
- `just db-stop`: Stop Supabase services
- `just setup-dev`: Initial setup for dev environment
- `just gen-types`: Regenerate OpenAPI JSON and TypeScript types from API DTOs
</operational_commands>

<quality_requirements>
- **Zero Warnings**: `just lint` must pass on all crates.
- **Fast Test Integrity**: `just test` must pass without `DATABASE_URL`.
- **DB Test Integrity**: `just test-db` must pass when a Postgres `DATABASE_URL` is available.
- **Contract Integrity**: `just gen-types` must leave `backend/docs/openapi.json` and `frontend/src/types/schema.ts` up to date.
</quality_requirements>

<architecture_rules>
- REST, CalDAV, bot, and worker code are adapters. Calendar mutations must go through `CalendarService`.
- Frontend types are generated from API DTOs, not storage rows.
- Event create/update requests use the explicit `timing` union: `timed` or `all_day`.
- Outbox messages use typed Rust payloads; do not build production outbox JSON by hand.
</architecture_rules>

</televent_rules>
