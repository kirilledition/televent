<objective>
Implement bot event creation (Blocker #1 for MVP)

Users should be able to create calendar events by sending multi-line text messages to the Telegram bot. Currently, `/create` only shows instructions but nothing happens when users send event details.

This is a critical MVP feature - users cannot create events via Telegram without this.
</objective>

<context>
**Project**: Televent - Telegram-native calendar with CalDAV sync
**Tech stack**: Rust, Teloxide (Telegram bot), SQLx (Postgres), Axum
**Key files to examine**:
- `@crates/bot/src/handlers.rs` - existing command handlers (pattern to follow)
- `@crates/bot/src/main.rs` - bot setup and message dispatch
- `@crates/bot/src/db.rs` - database operations (has `create_event` or similar)
- `@crates/bot/src/commands.rs` - command definitions
- `@docs/MVP_BLOCKERS_AND_PLAN.md` - detailed implementation plan
- `@CLAUDE.md` - project conventions (NO unwrap, use tracing, etc.)

**Current behavior**:
1. User runs `/create` → Bot shows format instructions
2. User sends event details → Nothing happens (no handler)

**Desired behavior**:
1. User runs `/create` → Bot shows format instructions
2. User sends multi-line event details → Bot creates event and confirms
</context>

<requirements>
**Input Format** (multi-line):
```
Event Title
tomorrow at 2pm
60
Conference Room A
```
Lines:
1. Event title (required)
2. Date/time - natural language (required) - use chrono-english for parsing
3. Duration in minutes (optional, default: 60)
4. Location (optional)

**Parsing Requirements**:
- Use `chrono-english` crate for flexible date parsing ("tomorrow 2pm", "next Monday 10:00", "2026-01-25 14:00")
- Stateless parsing: any non-command message with 2+ lines should be attempted as event creation
- Generate unique UID (UUID format) for each event
- Calculate end time from start + duration

**Error Handling**:
- If parsing fails, send helpful error message showing what was wrong
- Include correct format example in error messages
- Use `tracing::error!` for logging failures (never println!)

**Database Integration**:
- Look up user's calendar by telegram_id
- Call existing `db.create_event()` or equivalent
- Event must sync to CalDAV (verify existing event creation flow)
</requirements>

<implementation>
**Step 1: Add chrono-english dependency**
```bash
cargo add --package bot chrono-english
```

**Step 2: Create event parser module** (`crates/bot/src/event_parser.rs`)
- Define `ParsedEvent` struct with: title, start (DateTime), duration_minutes, location (Option)
- Implement `parse_event_message(text: &str) -> Result<ParsedEvent, ParseError>`
- Use chrono-english for date parsing
- Return typed errors for each failure case (missing title, invalid date, etc.)

**Step 3: Add message handler** (`crates/bot/src/handlers.rs`)
- Create `pub async fn handle_text_message(bot: Bot, msg: Message, db: BotDb) -> Result<()>`
- Skip if message starts with `/` (command, not event)
- Skip if fewer than 2 lines (not an event attempt)
- Parse message using event_parser
- On success: create event in DB, send confirmation
- On failure: send helpful error with format example

**Step 4: Wire up message handler** (`crates/bot/src/main.rs`)
- Add message handler that catches non-command text
- Route to `handle_text_message`
- Preserve existing command routing

**Patterns to follow** (from existing handlers.rs):
- Use `msg.from.as_ref()?.id.0 as i64` for telegram_id
- Use `bot.send_message(msg.chat.id, response).parse_mode(ParseMode::Html).await?`
- Use `tracing::info!()` for successful operations
- Return `Result<()>` with anyhow

**Constraints** (from CLAUDE.md):
- NO unwrap()/expect() - use `?` operator or explicit error handling
- NO println! - use tracing macros
- Use proper error types (thiserror for library code, anyhow for handlers)
</constraints>
</implementation>

<output>
Create/modify files:
- `./crates/bot/src/event_parser.rs` - new file with parsing logic
- `./crates/bot/src/handlers.rs` - add handle_text_message function
- `./crates/bot/src/main.rs` - wire up message handler
- `./crates/bot/src/lib.rs` - add event_parser module declaration
- `./crates/bot/Cargo.toml` - add chrono-english (via cargo add)
</output>

<verification>
Before declaring complete, verify:

1. **Build check**:
   ```bash
   cargo build --package bot
   ```

2. **Clippy check** (project requires zero warnings):
   ```bash
   cargo clippy --package bot -- -D warnings
   ```

3. **Unit tests** - add tests for event_parser:
   - Test valid multi-line input parses correctly
   - Test natural language dates ("tomorrow 2pm", "next Monday 10:00")
   - Test missing optional fields use defaults (60min duration, no location)
   - Test error cases return appropriate errors

4. **Integration verification** (manual):
   - Start bot: `just dev-bot`
   - Send `/create` - should show instructions
   - Send multi-line event text - should create event and confirm
   - Run `/today` or `/tomorrow` - should show the created event
</verification>

<success_criteria>
- [ ] `cargo build --package bot` succeeds
- [ ] `cargo clippy --package bot -- -D warnings` passes with zero warnings
- [ ] Event parser correctly handles multi-line input with natural language dates
- [ ] Non-command messages with 2+ lines trigger event creation attempt
- [ ] Created events appear in `/today`, `/tomorrow`, `/week` commands
- [ ] Error messages are helpful and show correct format
- [ ] No unwrap()/expect() calls in new code
- [ ] All logging uses tracing macros
</success_criteria>
