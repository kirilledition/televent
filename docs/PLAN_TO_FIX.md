# **Plan to Fix & Improvements for Televent**

This document outlines a comprehensive plan to address code review findings, focusing on architectural simplification, stability, security, and best practices.

## **Phase 1: Architectural Simplification (The "Tiny Crates" Fix)**

**Goal:** Reduce build times, complexity, and boilerplate by merging small crates.

1. **Merge crates/mailer into crates/worker:**  
   * **Action:** Move all logic from crates/mailer/src/lib.rs into crates/worker/src/mailer.rs. Delete the crates/mailer directory.  
   * **Why:** mailer is likely just a wrapper around lettre. It doesn't justify a separate compilation unit and Cargo.toml maintenance overhead. It's an implementation detail of the worker.  
2. **Consolidate Configuration:**  
   * **Action:** Create a crates/core/src/config.rs that handles loading .env and parsing common config (DB\_URL, Redis, etc.).  
   * **Why:** Currently api, bot, and worker likely duplicate this logic. Having a single source of truth in core prevents drift and bugs.  
3. **Evaluate crates/web Necessity:**  
   * **Action:** If crates/web is just static assets or a tiny Wasm shim, verify if it can be served directly by crates/api or if it truly needs a separate crate structure at this stage. (Assuming we keep it for Dioxus scaling, but keep it minimal).

## **Phase 2: Code Quality & Linting**

**Goal:** Enforce "Top Notch" code quality automatically.

1. **Strict Clippy Configuration:**  
   * **Action:** Add a root clippy.toml or configure \#\!\[warn(clippy::all, clippy::pedantic)\] in the crate roots.  
   * **Specific Rules:**  
     * clippy::unwrap\_used: Forbid .unwrap() in production code. Force proper error handling.  
     * clippy::expect\_used: Review all .expect() calls.  
     * clippy::nursery: Enable selected nursery lints for potential bugs.  
   * **Why:** Catches subtle bugs and enforces idiomatic Rust better than the default settings.  
2. **Clean up .gitignore:**  
   * **Action:** Consolidate .gitignore rules into the root .gitignore. Ensure .env is ignored at the root.  
   * **Why:** Reduces clutter and prevents accidental commits of secrets.

## **Phase 3: Core Domain Hardening**

**Goal:** Improve type safety and remove "Not Invented Here" code.

1. **Stronger Types:**  
   * **Action:** In crates/core/src/models.rs, replace String with enums for fields like status (e.g., EventStatus::Confirmed, EventStatus::Tentative).  
   * **Why:** Prevents invalid states (e.g., "status: 'maybe'") and makes logic clearer.  
2. **Fix Timezone Handling:**  
   * **Action:** Remove custom CSV loading in crates/core/src/timezone.rs. Use the chrono-tz crate.  
   * **Why:** Timezones are complex. Custom implementations are prone to bugs (DST, historical changes).  
3. **Use Standard Recurrence Parsing:**  
   * **Action:** Replace custom RecurrenceRule parsing in crates/core/src/recurrence.rs with the rrule crate.  
   * **Why:** RRULE spec (RFC 5545\) is complex. A library handles edge cases correctly.

## **Phase 4: Database & Migrations**

**Goal:** Ensure performance and data integrity.

1. **Add Missing Indexes:**  
   * **Action:** Create a new migration to add indexes:  
     * events(user\_id, dtstart) for calendar queries.  
     * outbox\_messages(status, created\_at) for the worker queue.  
   * **Why:** Essential for query performance as data grows.

## **Phase 5: API Security & Robustness**

**Goal:** Secure the application and handle errors gracefully.

1. **Restrict CORS:**  
   * **Action:** In crates/api/src/main.rs, change allow\_origin(Any) to allow only specific origins (e.g., the frontend URL).  
   * **Why:** Prevents unauthorized websites from making requests to the API.  
2. **Optimize Authentication:**  
   * **Action:** In crates/api/src/middleware/caldav\_auth.rs, implement a caching layer for authentication or use sessions/tokens instead of hashing passwords on every request.  
   * **Why:** Argon2 is CPU-intensive. Frequent CalDAV polling will overload the server.  
3. **Standardize Error Handling:**  
   * **Action:** Ensure all routes map errors to appropriate HTTP status codes (e.g., 400 for bad input, 404 for not found) instead of generic 500s. Use thiserror for library errors and anyhow for top-level application errors, mapping them to Axum responses.

## **Phase 6: CalDAV Reliability**

**Goal:** Ensure compatibility with calendar clients.

1. **Secure XML Parsing:**  
   * **Action:** Configure quick-xml to disable external entity processing (XXE protection) in crates/api/src/routes/caldav\_xml.rs.  
   * **Why:** Prevents security vulnerabilities.  
2. **Robust XML Generation:**  
   * **Action:** Use a library or a strict builder pattern for generating XML responses. Validate generated XML against CalDAV specs.  
   * **Why:** Manual string formatting is brittle and prone to syntax errors.

## **Phase 7: Worker & Concurrency**

**Goal:** Ensure reliable background processing.

1. **Fix Race Condition:**  
   * **Action:** Update the SQL query in crates/worker to use FOR UPDATE SKIP LOCKED when fetching jobs.  
   * **Why:** Allows multiple workers to run without processing the same job twice.  
2. **Implement Retry Logic:**  
   * **Action:** Ensure the worker retries failed jobs a limited number of times before marking them as failed/dead-letter.  
   * **Why:** Transient errors (network blips) shouldn't cause emails to be lost forever.

## **Phase 8: Testing**

**Goal:** Verify critical functionality.

1. **Add Integration Tests:**  
   * **Action:** Write tests that simulate a CalDAV client (using a library) interacting with the API.  
   * **Why:** Verifies that the server actually works with real-world clients.