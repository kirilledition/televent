-- Televent reset baseline schema.
--
-- Calendar sync invariants are application-owned. The database enforces data
-- shape and generic timestamps only.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TYPE event_status AS ENUM (
    'CONFIRMED',
    'TENTATIVE',
    'CANCELLED'
);

CREATE TYPE outbox_status AS ENUM (
    'pending',
    'processing',
    'completed',
    'failed'
);

CREATE TYPE attendee_role AS ENUM (
    'ORGANIZER',
    'ATTENDEE'
);

CREATE TYPE attendee_status AS ENUM (
    'NEEDS-ACTION',
    'ACCEPTED',
    'DECLINED',
    'TENTATIVE'
);

-- -----------------------------------------------------------------------------
-- 2. Shared Functions
-- -----------------------------------------------------------------------------

-- Automatically updates the updated_at timestamp on row modifications
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION update_updated_at() IS
    'Trigger function to automatically update updated_at timestamp';

CREATE TABLE users (
    telegram_id BIGINT PRIMARY KEY,
    telegram_username TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    sync_token BIGINT NOT NULL DEFAULT 0,
    ctag BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE UNIQUE INDEX idx_users_telegram_username
    ON users(lower(telegram_username));

-- Triggers
CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Documentation
COMMENT ON TABLE users IS
    'Telegram users who have registered with Televent (user = calendar)';
COMMENT ON COLUMN users.telegram_id IS
    'Telegram user ID - permanent unique identifier';
COMMENT ON COLUMN users.telegram_username IS
    'Telegram username (handle) - can change, used for CalDAV URLs';
COMMENT ON COLUMN users.timezone IS
    'IANA timezone string (e.g., Asia/Singapore, Europe/London)';
COMMENT ON COLUMN users.sync_token IS
    'RFC 6578 sync token. Numeric version incremented by application transactions only';
COMMENT ON COLUMN users.ctag IS
    'Collection tag version for change detection. Mirrors sync_token for beta';

CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,
    uid TEXT NOT NULL,
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    start TIMESTAMPTZ,
    "end" TIMESTAMPTZ,
    start_date DATE,
    end_date DATE,
    is_all_day BOOLEAN NOT NULL DEFAULT FALSE,
    status event_status NOT NULL DEFAULT 'CONFIRMED',
    rrule TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    version INTEGER NOT NULL DEFAULT 1,
    sync_version BIGINT NOT NULL DEFAULT 0,
    etag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT check_event_type_integrity CHECK (
        (is_all_day = true AND
         start_date IS NOT NULL AND end_date IS NOT NULL AND
         end_date > start_date AND
         start IS NULL AND "end" IS NULL)
        OR
        (is_all_day = false AND
         start_date IS NULL AND end_date IS NULL AND
         start IS NOT NULL AND "end" IS NOT NULL AND
         "end" > start)
    )
);

-- Indexes
CREATE UNIQUE INDEX idx_events_user_uid
    ON events(user_id, uid);

CREATE INDEX idx_events_time_range
    ON events(user_id, start, "end")
    WHERE NOT is_all_day;

CREATE INDEX idx_events_start_date
    ON events(user_id, start_date)
    WHERE is_all_day;

CREATE INDEX idx_events_user_sync_version
    ON events(user_id, sync_version);

-- Triggers
CREATE TRIGGER events_updated_at
    BEFORE UPDATE ON events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Documentation
COMMENT ON TABLE events IS
    'Calendar events belonging to users';
COMMENT ON COLUMN events.user_id IS
    'Owner user telegram_id';
COMMENT ON COLUMN events.uid IS
    'iCalendar UID (stable across syncs)';
COMMENT ON COLUMN events.etag IS
    'HTTP ETag for conflict detection';
COMMENT ON COLUMN events.version IS
    'iCalendar SEQUENCE / optimistic version, incremented by application use cases';
COMMENT ON COLUMN events.sync_version IS
    'Collection-wide sync version assigned by application use cases';
COMMENT ON COLUMN events.is_all_day IS
    'Distinguishes all-day events (DATE) from timed events (TIMESTAMPTZ)';
COMMENT ON CONSTRAINT check_event_type_integrity ON events IS
    'Ensures events are either valid timed ranges (start/end) OR valid all-day ranges (start_date/end_date), never both';

CREATE TABLE event_tombstones (
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,
    uid TEXT NOT NULL,
    sync_version BIGINT NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, uid)
);

CREATE INDEX idx_event_tombstones_user_sync_version
    ON event_tombstones(user_id, sync_version);

COMMENT ON TABLE event_tombstones IS
    'Deleted CalDAV resources returned as 404 responses by sync-collection';

CREATE TABLE event_attendees (
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    user_id BIGINT,
    role attendee_role NOT NULL DEFAULT 'ATTENDEE',
    status attendee_status NOT NULL DEFAULT 'NEEDS-ACTION',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, email)
);

-- Indexes
CREATE INDEX idx_event_attendees_event
    ON event_attendees(event_id);

CREATE INDEX idx_event_attendees_telegram
    ON event_attendees(user_id)
    WHERE user_id IS NOT NULL;

-- Triggers
CREATE TRIGGER event_attendees_updated_at
    BEFORE UPDATE ON event_attendees
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Documentation
COMMENT ON TABLE event_attendees IS
    'Event attendees/participants';
COMMENT ON COLUMN event_attendees.user_id IS
    'Telegram ID for internal users, NULL for external invites';
COMMENT ON COLUMN event_attendees.email IS
    'Email address (required for CalDAV ATTENDEE property)';

CREATE TABLE device_passwords (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,
    device_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

-- Indexes
CREATE INDEX idx_device_passwords_user
    ON device_passwords(user_id);

CREATE INDEX idx_device_passwords_hash
    ON device_passwords(password_hash);

-- Documentation
COMMENT ON TABLE device_passwords IS
    'Device-specific passwords for CalDAV authentication';
COMMENT ON COLUMN device_passwords.password_hash IS
    'Argon2id hash of device password';
COMMENT ON COLUMN device_passwords.last_used_at IS
    'Last successful authentication timestamp (for security auditing)';

CREATE TABLE outbox_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind TEXT NOT NULL,
    payload JSONB NOT NULL,
    dedupe_key TEXT,
    status outbox_status NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_message TEXT,
    CONSTRAINT check_outbox_kind CHECK (
        kind IN (
            'invite_notification',
            'telegram_notification',
            'external_email_deferred',
            'rsvp_notification'
        )
    )
);

-- Indexes (partial indexes for efficiency)
CREATE INDEX idx_outbox_pending
    ON outbox_messages(status, scheduled_at)
    WHERE status = 'pending';

CREATE INDEX idx_outbox_failed
    ON outbox_messages(status, created_at)
    WHERE status = 'failed';

CREATE UNIQUE INDEX idx_outbox_dedupe
    ON outbox_messages(dedupe_key)
    WHERE dedupe_key IS NOT NULL;

CREATE TRIGGER outbox_messages_updated_at
    BEFORE UPDATE ON outbox_messages
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Documentation
COMMENT ON TABLE outbox_messages IS
    'Transactional outbox for reliable async message processing';
COMMENT ON COLUMN outbox_messages.kind IS
    'Typed message discriminator for routing to appropriate handlers';
COMMENT ON CONSTRAINT check_outbox_kind ON outbox_messages IS
    'Restricts outbox messages to Rust OutboxKind discriminators';
COMMENT ON COLUMN outbox_messages.payload IS
    'JSON payload decoded by Rust typed outbox enums';
COMMENT ON COLUMN outbox_messages.dedupe_key IS
    'Optional idempotency key for transactional enqueue operations';
COMMENT ON COLUMN outbox_messages.retry_count IS
    'Number of processing attempts (for exponential backoff)';
