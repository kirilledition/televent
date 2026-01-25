-- Consolidated Initialization Migration
-- This file squashes all previous migrations into a clean, unified schema.

-- 1. Create Enums
CREATE TYPE event_status AS ENUM ('CONFIRMED', 'TENTATIVE', 'CANCELLED');
CREATE TYPE outbox_status AS ENUM ('pending', 'processing', 'completed', 'failed');
CREATE TYPE attendee_role AS ENUM ('ORGANIZER', 'ATTENDEE');
CREATE TYPE participation_status AS ENUM ('NEEDS-ACTION', 'ACCEPTED', 'DECLINED', 'TENTATIVE');

-- 2. Create users table (user = calendar)
CREATE TABLE users (
    telegram_id BIGINT PRIMARY KEY,
    telegram_username TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    -- Calendar properties
    calendar_name TEXT NOT NULL DEFAULT 'My Calendar',
    calendar_color TEXT NOT NULL DEFAULT '#3b82f6',
    sync_token TEXT NOT NULL DEFAULT '0',
    ctag TEXT NOT NULL DEFAULT '0',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fast lookup by username (case-insensitive)
CREATE UNIQUE INDEX idx_users_telegram_username ON users(lower(telegram_username));

-- Update updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments for documentation
COMMENT ON TABLE users IS 'Telegram users who have registered with Televent (user = calendar)';
COMMENT ON COLUMN users.telegram_id IS 'Telegram user ID - permanent unique identifier';
COMMENT ON COLUMN users.telegram_username IS 'Telegram username (handle) - can change, used for CalDAV URLs';
COMMENT ON COLUMN users.timezone IS 'IANA timezone string (e.g., Asia/Singapore, Europe/London)';
COMMENT ON COLUMN users.sync_token IS 'RFC 6578 sync token for CalDAV sync-collection';
COMMENT ON COLUMN users.ctag IS 'Collection tag for change detection (timestamp-based)';

-- 3. Create events table
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,
    uid TEXT NOT NULL,
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    -- Time events
    start TIMESTAMPTZ,
    "end" TIMESTAMPTZ,
    -- All-day events
    start_date DATE,
    end_date DATE,
    is_all_day BOOLEAN NOT NULL DEFAULT FALSE,
    status event_status NOT NULL DEFAULT 'CONFIRMED',
    rrule TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    version INTEGER NOT NULL DEFAULT 1,
    etag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- XOR Integrity: Must be either a Time Event or an All-Day Event
    CONSTRAINT check_event_type_integrity CHECK (
        (is_all_day = true  AND start_date IS NOT NULL AND end_date IS NOT NULL AND start IS NULL AND "end" IS NULL) OR
        (is_all_day = false AND start_date IS NULL AND end_date IS NULL AND start IS NOT NULL AND "end" IS NOT NULL)
    )
);

-- Unique constraint: one UID per user
CREATE UNIQUE INDEX idx_events_user_uid ON events(user_id, uid);

-- Indices for queries
CREATE INDEX idx_events_time_range ON events(user_id, start, "end") WHERE NOT is_all_day;
CREATE INDEX idx_events_start_date ON events(user_id, start_date) WHERE is_all_day;

-- Update trigger for events
CREATE TRIGGER events_updated_at
    BEFORE UPDATE ON events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE events IS 'Calendar events belonging to users';
COMMENT ON COLUMN events.user_id IS 'Owner user telegram_id';
COMMENT ON COLUMN events.uid IS 'iCalendar UID (stable across syncs)';
COMMENT ON COLUMN events.etag IS 'HTTP ETag for conflict detection';

-- 4. Create event_attendees table
CREATE TABLE event_attendees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    telegram_id BIGINT,
    role attendee_role NOT NULL DEFAULT 'ATTENDEE',
    status participation_status NOT NULL DEFAULT 'NEEDS-ACTION',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indices
CREATE INDEX idx_event_attendees_event ON event_attendees(event_id);
CREATE INDEX idx_event_attendees_telegram ON event_attendees(telegram_id) WHERE telegram_id IS NOT NULL;
CREATE UNIQUE INDEX idx_event_attendees_unique ON event_attendees(event_id, email);

-- Update trigger for attendees
CREATE TRIGGER event_attendees_updated_at
    BEFORE UPDATE ON event_attendees
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE event_attendees IS 'Event attendees/participants';
COMMENT ON COLUMN event_attendees.telegram_id IS 'Telegram ID for internal users, NULL for external invites';

-- 5. Create device_passwords table
CREATE TABLE device_passwords (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,
    device_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

-- Index for auth lookups
CREATE INDEX idx_device_passwords_user ON device_passwords(user_id);

-- Comments
COMMENT ON TABLE device_passwords IS 'Device-specific passwords for CalDAV authentication';
COMMENT ON COLUMN device_passwords.password_hash IS 'Argon2id hash of device password';

-- 6. Create outbox_messages table
CREATE TABLE outbox_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status outbox_status NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_message TEXT
);

-- Indices
CREATE INDEX idx_outbox_pending ON outbox_messages(status, scheduled_at) WHERE status = 'pending';
CREATE INDEX idx_outbox_failed ON outbox_messages(status, created_at) WHERE status = 'failed';

-- Comments
COMMENT ON TABLE outbox_messages IS 'Transactional outbox for async message processing';
