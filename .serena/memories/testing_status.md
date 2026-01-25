# Televent Testing Status

## Coverage Goals
- **Target**: 80%+ test coverage for all major crates (`api`, `bot`, `core`, `worker`).
- **Tooling**: `cargo-llvm-cov` is used to generate HTML reports.
  - `just test-coverage` runs the workspace-wide check.

## Testing Strategy
- **Unit Tests**: Located within each crate for logic verification (e.g., date parsing, RRule expansion).
- **Integration Tests**:
    - **API**: Testing Axum routes and CalDAV XML responses.
    - **Bot**: Using `teloxide_tests` to simulate user interactions and verify handler responses.
    - **Database**: Using `sqlx::test` for real database integration testing during development.

## Quality Standards
- No `unwrap()` or `expect()`—explicit error handling with `?` or `anyhow::Result`.
- Zero compiler warnings or Clippy warnings.
- `just lint` must pass before any commit.
