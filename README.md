# Televent

Telegram-native calendar management with CalDAV sync. No Google/Microsoft account required.

## Quick Start

1. **Open in dev container** - Everything sets up automatically
2. **Add your Telegram bot token** to `.env` (get from @BotFather)
3. **Run the bot:** `just bot`
4. **Message your bot** on Telegram

## Commands

```bash
# Run services
just bot          # Telegram bot
just api          # REST API + CalDAV server
just worker       # Background jobs

# Development (auto-reload)
just dev-bot      # Bot with auto-reload
just dev-api      # API with auto-reload

# Code quality
just fmt          # Format code
just lint         # Check code
just fix          # Auto-fix issues
just test         # Run tests
just ci           # Full CI check

# Database
just db-console   # PostgreSQL console
just db-tables    # Show tables
just db-reset     # Reset database

# See all commands
just --list
```

## Tech Stack

- **Backend:** Rust, Axum, SQLx (PostgreSQL), Teloxide
- **Frontend:** Dioxus (planned)
- **Dev:** Docker, Just, cargo-watch

## Project Structure

```
crates/
├── bot/        # Telegram bot (11 commands)
├── api/        # REST API + CalDAV server
├── worker/     # Background job processor
├── core/       # Domain models
├── mailer/     # Email sender
└── web/        # Web UI (planned)
```

## Features

✅ **Telegram Bot** - Create events, get reminders, natural language date parsing
✅ **Device Passwords** - CalDAV sync with Apple Calendar, Thunderbird, etc.
✅ **Background Worker** - Email notifications, scheduled jobs
🔄 **CalDAV Server** - Calendar sync (partial implementation)
⏳ **Web UI** - Coming soon
⏳ **GDPR Compliance** - Data export, account deletion (planned)

## Development

**Code Guidelines:** See [CLAUDE.md](CLAUDE.md)
**Architecture:** See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
**Development Log:** See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)

**Critical Rules:**
- NO `unwrap()`/`expect()` - use `?` or explicit error handling
- NO `println!` - use `tracing::info!` or `tracing::error!`
- Use newtypes for IDs (`UserId(Uuid)`, not raw `Uuid`)
- Always use `tokio` runtime

## Documentation

- [CLAUDE.md](CLAUDE.md) - Code style and rules
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - System architecture
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) - Development history
- [docs/bot_commands.md](docs/bot_commands.md) - Bot commands specification
- [docs/caldav_compliance.md](docs/caldav_compliance.md) - CalDAV implementation
- [docs/gdpr_procedures.md](docs/gdpr_procedures.md) - GDPR compliance

## Getting Your Bot Token

1. Open Telegram and message **@BotFather**
2. Send `/newbot`
3. Follow prompts to name your bot
4. Copy the token (format: `123456789:ABC-DEF...`)
5. Add to `.env`: `TELEGRAM_BOT_TOKEN=your_token_here`

## Testing

```bash
just test              # All tests
just test-crate bot    # Specific crate
just test-coverage     # With coverage
```

## License

See LICENSE file.
