# Televent Tech Stack

## Backend
- **Language**: Rust (Edition 2024, v1.93+)
- **Web Framework**: Axum (v0.8)
- **Database Access**: SQLx (v0.8) with Postgres
- **Bot Framework**: Teloxide (v0.17)
- **Async Runtime**: Tokio (v1.49)
- **Serialization**: Serde

## Database & Infrastructure
- **Development Database**: Supabase (via CLI)
- **Migrations**: SQLx Migrations + Supabase CLI
- **Tooling**: 
  - `Just` for task orchestration
  - `Nix` (flake.nix) for reproducible dev environments
  - `Docker` for containerized services

## APIs & Protocols
- **CalDAV**: RFC 4791,quick-xml for XML handling
- **Telegram Bot API**: Teloxide
- **Email**: Lettre (with tokio-rustls)

## Quality Assurance
- **Linting**: Clippy (pedantic/nursery enabled), `cargo fmt`
- **Testing**: `cargo test`, `teloxide_tests`
- **Coverage**: `cargo-llvm-cov`
