# GDPR Compliance Procedures

## Overview

Televent is GDPR compliant and provides users with full control over their data.

## Data We Collect

### User Data
- Telegram ID (required for authentication)
- Telegram username (optional, from Telegram)
- Timezone preference
- Account creation timestamp

### Calendar Data
- Events (title, description, location, date/time)
- Calendar metadata (name, color, sync tokens)
- Device passwords (Argon2id hashed)

### Audit Logs
- User actions (event_created, data_exported, account_deleted)
- IP addresses (for security purposes)
- User agents
- Retained for 2 years (legal requirement)

## User Rights

### 1. Right to Access
Users can request all their data via the `/export` command.

**Implementation**:
```rust
// GET /api/gdpr/export or /export bot command
// Returns JSON file with:
{
  "user": {...},
  "calendars": [...],
  "events": [...],
  "device_passwords": [...], // Hashed only
  "audit_log": [...],
  "exported_at": "2026-01-18T..."
}
```

**Response Time**: Instant (real-time export)

### 2. Right to Rectification
Users can modify their data via:
- Bot commands (`/create`, `/cancel`, `/timezone`)
- CalDAV client sync
- Web UI

### 3. Right to Erasure (Right to be Forgotten)
Users can delete their account via the `/delete_account` command.

**Implementation**:
1. Two-step confirmation (prevents accidents)
2. 30-day grace period (can restore via support)
3. Data moved to `deleted_users` table
4. After 30 days: permanent deletion

**SQL**:
```sql
-- Soft delete
INSERT INTO deleted_users (id, telegram_id, data_snapshot, deletion_requested_at, permanent_deletion_at)
SELECT id, telegram_id, jsonb_build_object(...), NOW(), NOW() + INTERVAL '30 days'
FROM users WHERE id = $1;

-- Cascade delete
DELETE FROM users WHERE id = $1;
```

**Permanent Deletion** (after 30 days):
```sql
DELETE FROM deleted_users WHERE permanent_deletion_at < NOW();
```

### 4. Right to Data Portability
Exported data is in JSON format (machine-readable).

Events can also be exported via CalDAV (iCalendar format).

### 5. Right to Object
Users can opt out of:
- Event reminders
- Daily digest
- (No marketing emails - we don't send any)

## Data Retention

| Data Type           | Retention Period | Notes                          |
| ------------------- | ---------------- | ------------------------------ |
| User accounts       | Until deleted    | User-controlled                |
| Events              | Until deleted    | User-controlled                |
| Audit logs          | 2 years          | Legal requirement              |
| Outbox messages     | 90 days          | Then archived or deleted       |
| Deleted accounts    | 30 days          | Grace period for recovery      |

## Data Processing

### Legal Basis
- **Contract**: Providing calendar service to users
- **Legitimate Interest**: Security (audit logs, IP tracking)

### Data Location
- EU users: Data stored in EU region
- All users: Can request data location via support

### Third-Party Services
- **Telegram**: Authentication only (no data shared)
- **PostgreSQL**: Database hosting (encrypted at rest)
- **Jaeger/Sentry**: Monitoring (no PII)

## Security Measures

1. **Encryption**:
   - HTTPS/TLS for all API traffic
   - Database encryption at rest
   - Argon2id for password hashing

2. **Access Control**:
   - Device passwords (per-device revocation)
   - JWT tokens (24h expiry)
   - Rate limiting

3. **Audit Trail**:
   - All data access logged
   - IP tracking for security
   - Anomaly detection (future)

## Data Breach Notification

In case of a data breach:
1. Users notified within 72 hours via Telegram
2. Breach details published on status page
3. Authorities notified (if required by law)

## Contact for GDPR Requests

- **Email**: privacy@televent.app
- **Response Time**: 30 days (as per GDPR)

## Implementation Checklist

- [x] Data export endpoint
- [x] Account deletion with grace period
- [ ] Privacy policy page
- [ ] Terms of service page
- [ ] Cookie consent (if web cookies used)
- [ ] Data breach notification workflow
- [ ] GDPR training for team
- [ ] Regular compliance audits
