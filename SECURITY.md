# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Security Best Practices

### Password Storage

All passwords are hashed using **Argon2id** with secure defaults:
- Memory: 19456 KiB (19 MiB)
- Iterations: 2
- Parallelism: 1
- Random salt per password

**Never** store plaintext passwords or use weak hashing algorithms.

### Authentication

#### Telegram Authentication
- Web UI uses Telegram Login Widget with HMAC-SHA256 validation
- Bot token is used as the secret key
- All init data must be validated before creating sessions
- Sessions use JWT tokens stored in httpOnly cookies (24h expiry)

#### CalDAV Authentication
- HTTP Basic Auth with device-specific passwords
- Username format: `telegram_id:device_password`
- Passwords are Argon2id hashed in the database
- Track last_used_at for suspicious activity detection

### CalDAV Security

#### ETag Generation
- ETags are SHA256 hashes of event content
- **Never** use timestamps for ETags (clock skew causes false conflicts)
- Format: lowercase hex string

#### Conflict Detection
- Use optimistic locking with version field
- Check If-Match header on PUT/DELETE operations
- Return 412 Precondition Failed on version mismatch

### Database Security

#### SQL Injection Prevention
- **Always** use parameterized queries with SQLx
- **Never** concatenate user input into SQL strings
- SQLx compile-time query validation prevents many injection attacks

#### Sensitive Data
- Device passwords: Argon2id hashed
- Audit logs: Track all security-relevant operations
- Connection strings: Store in environment variables, never in code

### API Security

#### Rate Limiting
- CalDAV endpoints: 100 requests/minute per user
- REST API: 300 requests/minute per user
- Bot: 20 messages/second (Teloxide throttle)

#### Input Validation
- Validate timezone strings against IANA database
- Validate RRULE strings against RFC 5545
- Sanitize all user input before storage
- Validate email addresses before sending
- Check event date ranges (end >= start)

#### Error Handling
- **Never** expose internal error details in API responses
- Log detailed errors with tracing for debugging
- Return generic error messages to clients
- Use anyhow::Context to add context without exposing internals

### GDPR Compliance

#### Data Minimization
- Only collect necessary data (telegram_id, username, timezone)
- No email addresses stored unless explicitly provided
- One calendar per user (reduces data collection)

#### Data Export
- Users can export all their data via `/export` command
- Export includes: user data, events, device passwords (hashed), audit logs
- All exports are logged in audit_log table

#### Data Deletion
- 30-day grace period for account deletion
- Soft delete to deleted_users table with encrypted snapshot
- Permanent deletion after grace period
- Cascade deletion of all user data

#### Audit Trail
- All security-relevant operations logged
- Retention: 2 years minimum
- Include: user_id, action, entity_type, entity_id, ip_address, user_agent

## Reporting a Vulnerability

If you discover a security vulnerability, please report it to:

**Email**: security@televent.app (or create a private security advisory on GitHub)

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

We will:
- Acknowledge receipt within 24 hours
- Provide a timeline for fix within 72 hours
- Credit you in the security advisory (unless you prefer to remain anonymous)
- Notify you when the fix is deployed

**Do not** publicly disclose the vulnerability until we've had a chance to fix it.

## Security Checklist for Contributors

Before submitting a PR:

- [ ] No hardcoded secrets or credentials
- [ ] All passwords hashed with Argon2id
- [ ] All SQL queries use parameterized queries (no string concatenation)
- [ ] User input is validated and sanitized
- [ ] Error messages don't expose internal details
- [ ] Authentication required for all sensitive endpoints
- [ ] Rate limiting applied to public endpoints
- [ ] GDPR considerations addressed (data minimization, export, deletion)
- [ ] Security-relevant operations logged to audit_log
- [ ] Tests cover security-critical code paths

## Known Security Considerations

### Clock Skew
- ETags use content hashing, not timestamps
- Sync tokens are server-side only (no clock dependency)
- Event times are always in UTC, converted on display

### Concurrent Access
- Optimistic locking prevents lost updates
- `FOR UPDATE SKIP LOCKED` in worker prevents duplicate processing
- Database transactions for multi-step operations

### Session Management
- JWT tokens expire after 24 hours
- httpOnly cookies prevent XSS attacks
- Secure flag in production (HTTPS only)
- SameSite=Strict to prevent CSRF

## Security Updates

We will publish security advisories for:
- Critical vulnerabilities (CVSS >= 9.0)
- High severity vulnerabilities (CVSS >= 7.0)
- Any vulnerability with known exploits

Subscribe to GitHub security advisories for notifications.
