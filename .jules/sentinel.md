## 2025-02-17 - [Timing Attack in Auth]
**Vulnerability:** User enumeration via timing discrepancies in CalDAV Basic Auth. Invalid users returned immediately, valid users triggered slow Argon2 verification.
**Learning:** Middleware handling expensive operations (hashing) must ensure uniform execution time regardless of user existence.
**Prevention:** Always perform a dummy verification (same work factor) when the happy path would have performed one, even if the user or data is missing.

## 2025-02-18 - [API IDOR Vulnerability]
**Vulnerability:** Unprotected /api endpoints allowed IDOR; any user could manipulate any calendar/event by guessing UUIDs.
**Learning:** Middleware alone is insufficient; handlers must explicitly verify ownership of resources (calendar_id, event_id) against the authenticated user.
**Prevention:** Implement `is_owner` checks in DB layer and enforce them in every handler that accepts an ID.
