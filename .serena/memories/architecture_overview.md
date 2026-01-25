# Televent Architecture Overview

The project is structured as a Rust workspace (monorepo) with several interconnected crates.

## Crates
- `crates/core`: Shared logic, business models, timezone handling, and iCalendar/RRule processing.
- `crates/api`: Axum-based web server implementing the CalDAV protocol and event management APIs.
- `crates/bot`: Telegram bot implementation using Teloxide, providing a conversational interface for event management.
- `crates/worker`: Background worker for tasks like reminder processing or sync synchronization.
- `crates/server`: The "unified server" that runs all services (`api`, `bot`, `worker`) in a single process—default for current deployment.
- `frontend`: Next.js application serving as the Telegram Mini App and web dashboard.

## Key Components
- **Routes (`crates/api/src/routes`)**:
    - `caldav.rs` / `caldav_xml.rs`: CalDAV protocol implementation.
    - `events.rs`: JSON API for event management.
    - `devices.rs`: Device management (web/mobile authentication).
- **Bot Handlers (`crates/bot/src/handlers.rs`)**: Manages interactions with Telegram users.
- **Unified Entry Point (`crates/server/src/main.rs`)**: Orchestrates the startup of Axum and Teloxide runtimes.

## Data Flow
- Users interact via Telegram (Bot) or CalDAV consumers (Web/iOS/Android).
- All components share a common PostgreSQL database managed through SQLx.
- Core business rules are centralized in `crates/core` to ensure consistency between the Bot and API.
