## 2025-02-17 - [Timing Attack in Auth]
**Vulnerability:** User enumeration via timing discrepancies in CalDAV Basic Auth. Invalid users returned immediately, valid users triggered slow Argon2 verification.
**Learning:** Middleware handling expensive operations (hashing) must ensure uniform execution time regardless of user existence.
**Prevention:** Always perform a dummy verification (same work factor) when the happy path would have performed one, even if the user or data is missing.

## 2025-02-18 - [Replay Attack in Telegram Auth]
**Vulnerability:** Telegram initData validation verified the signature but ignored auth_date, allowing replay attacks.
**Learning:** Signature verification proves authenticity but not freshness. Stateless auth tokens/data always need an expiration check.
**Prevention:** Verify auth_date is within a validity window (e.g., 24h) immediately after signature validation.

## 2025-05-18 - [IDOR in Event Access]
**Vulnerability:** Event API endpoints (GET, PUT, DELETE) extracted the event ID from the path and queried the database solely by ID, ignoring the authenticated user context.
**Learning:** Middleware authentication does not imply authorization at the data access layer.
**Prevention:** Database functions must accept `user_id` and enforce it in the WHERE clause (e.g., `WHERE id = $1 AND user_id = $2`).

## 2025-02-18 - [DoS in Auth Middleware Order]
**Vulnerability:** Rate limiting middleware was placed *after* authentication middleware. Attackers could bypass rate limits by sending invalid credentials, triggering expensive Argon2 verification and causing DoS.
**Learning:** In Axum/Tower, `.layer(A).layer(B)` results in `B` wrapping `A`. To protect Auth, Rate Limit must be the *outer* layer (added *after* Auth in code).
**Prevention:** Always verify middleware execution order. Place cheap, IP-based rate limiting *before* expensive authentication or database logic.

## 2025-05-19 - [Blocking Async Runtime with Argon2]
**Vulnerability:** `verify_password` used Argon2id synchronously within an async handler/middleware, blocking the Tokio executor and causing potential DoS.
**Learning:** CPU-intensive operations like password hashing/verification must be offloaded to `tokio::task::spawn_blocking` in async applications.
**Prevention:** Wrap all Argon2 calls in `spawn_blocking` to prevent starving the async runtime.
