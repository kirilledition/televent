## 2025-02-17 - [Timing Attack in Auth]
**Vulnerability:** User enumeration via timing discrepancies in CalDAV Basic Auth. Invalid users returned immediately, valid users triggered slow Argon2 verification.
**Learning:** Middleware handling expensive operations (hashing) must ensure uniform execution time regardless of user existence.
**Prevention:** Always perform a dummy verification (same work factor) when the happy path would have performed one, even if the user or data is missing.
