# Televent Codebase Audit - Complete Findings

## Executive Summary
This is a Rust monorepo for a Telegram-based calendar app with CalDAV sync. Multiple components contain significant dead code, and some authentication middleware is entirely unused.

---

## RESOLVED ISSUES

### 1. **DEAD TELEGRAM AUTH MIDDLEWARE REMOVED** ✅
- **Status**: Resolved
- **Action**: Deleted `crates/api/src/middleware/telegram_auth.rs`.
- **Note**: Authentication is now handled via CalDAV basic auth and Supabase-ready patterns.

### 2. **ABANDONED DIALOGUE SYSTEM REMOVED** ✅
- **Status**: Resolved
- **Action**: Deleted `crates/bot/src/dialogue.rs`.
- **Note**: The bot now uses a more streamlined command and handler system without the abandoned dialogue state machine.

---

## REMAINING MAJOR ISSUES

### 3. **DEVICES ROUTE UNFINISHED** ⚠️ MAJOR
**File**: `crates/api/src/routes/devices.rs`
- **Status**: Basic CRUD exists but needs better validation and integration with Auth.

---

## MIGRATION STATUS
- Switched to Supabase for local and production database management.
- SQLx migrations are still used for schema control.
- `just db-reset` and `just db-start` now leverage Supabase CLI.

---

## RECOMMENDATIONS (Priority Order)


### P0 - Remove Dead Code
1. Delete `crates/bot/src/dialogue.rs` entirely
2. Delete `crates/api/src/middleware/telegram_auth.rs` entirely
3. Delete `BotDb::get_event()`, `user_has_calendar()`, `create_event()` methods
4. Delete `get_calendar_by_user()` function in calendars.rs

### P1 - Security/Compliance
1. Implement or document deleted event tracking for RFC 6578 compliance
2. Implement rate limiting (currently TODO)
3. Document/implement Telegram OAuth if web UI needs it
4. Verify JWT secret is not used without middleware

### P2 - Features
1. Implement ICS export (currently placeholder)
2. Complete worker implementations

### P3 - Code Quality
1. Fix all compilation warnings
2. Document why ICS export is not implemented
3. Add sqlx::prepare queries for compile-time verification

---

## VERDICT

**Overall Code Health**: 8/10
- Significant dead code removed (dialogue system, unused middleware).
- Modernized with Supabase integration.
- Improved layout and workspace management.

**Production Readiness**: 7/10
- Core functionality stable.
- Supabase integration provides a robust DB backend.
- Still needs full RFC compliance verification for CalDAV.
