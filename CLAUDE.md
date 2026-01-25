<televent_rules>

<critical_rules>

- **NO unwrap()/expect()**: Use `?` or explicit error handling. Why: We handle calendars - data loss from panics is unacceptable.
- **NO println!**: Use `tracing::info!` or `tracing::error!`. Why: We need structured logs for debugging production CalDAV sync issues.
- **Type Safety**: Use newtypes (`UserId(Uuid)`, not raw `Uuid`). Why: CalDAV spec has many ID types - type confusion causes sync corruption.
- **Async Runtime**: Always `tokio`. Why: Entire stack (Axum, SQLx, Teloxide) is tokio-based.
</critical_rules>

<tech_stack>
- **Backend**: Axum + SQLx (Postgres) + Teloxide
- **Tooling**: Rust, Just, Cargo, Docker, Nix
</tech_stack>

<operational_commands>

- `just lint`: Check code quality
- `just run`: Run app
- `just test`: Run tests
- `just test-coverage`: Run tests with coverage report
- `just db-reset`: Reset database (drop, create, migrate)
- `just db-start`: Start PostgreSQL service via Docker
- `just db-status`: Check PostgreSQL status
- `just db-stop`: Stop PostgreSQL service
- `just setup-dev`: Initial setup for dev container
</operational_commands>

<quality_requirements>
- **Zero Warnings**: `just lint` must pass on all crates.
- **Test Integrity**: All tests must pass via `just test`
</quality_requirements>

<tool_use>

- **`serena` project** - Rust development - Activate the project
- **`db` MCP** - Database access - Use tools
- **`context7` MCP** - Library or tool documentation - Use for documentation
</tool_use>

</televent_rules>