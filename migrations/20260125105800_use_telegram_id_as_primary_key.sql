-- Migration: Use Telegram ID as Primary Key
-- This migration restructures the database to use telegram_id (BIGINT) as the primary key
-- for users, eliminating artificial UUIDs and merging calendar data into the users table.

-- Drop all existing tables in correct order (respecting foreign keys)
DROP TABLE IF EXISTS event_attendees CASCADE;
DROP TABLE IF EXISTS deleted_events CASCADE;
DROP TABLE IF EXISTS device_passwords CASCADE;
DROP TABLE IF EXISTS events CASCADE;
DROP TABLE IF EXISTS calendars CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- Drop the event_status type if it exists (we'll recreate it)
DROP TYPE IF EXISTS event_status CASCADE;

-- Create users table with telegram_id as primary key
-- Calendar data is merged into this table since each user has exactly one calendar
CREATE TABLE users (
    telegram_id BIGINT PRIMARY KEY,
    telegram_username TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    -- Calendar properties (merged from calendars table)
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
COMMENT ON COLUMN users.timezone IS 'IANA timezone string (e.g., Asia/Singapore)';
COMMENT ON COLUMN users.calendar_name IS 'Display name for the user calendar';
COMMENT ON COLUMN users.calendar_color IS 'Hex color code for UI display';
COMMENT ON COLUMN users.sync_token IS 'RFC 6578 sync token for CalDAV sync-collection';
COMMENT ON COLUMN users.ctag IS 'Collection tag for change detection';

-- Create event_status enum
CREATE TYPE event_status AS ENUM ('CONFIRMED', 'TENTATIVE', 'CANCELLED');

-- Create events table referencing user's telegram_id directly
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,
    uid TEXT NOT NULL,
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    start TIMESTAMPTZ NOT NULL,
    "end" TIMESTAMPTZ NOT NULL,
    is_all_day BOOLEAN NOT NULL DEFAULT FALSE,
    status event_status NOT NULL DEFAULT 'CONFIRMED',
    rrule TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    version INTEGER NOT NULL DEFAULT 1,
    etag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint: one UID per user
CREATE UNIQUE INDEX idx_events_user_uid ON events(user_id, uid);

-- Index for time-based queries
CREATE INDEX idx_events_time_range ON events(user_id, start, "end");

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

-- Create device_passwords table for CalDAV authentication
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

-- Create deleted_events table for CalDAV sync
CREATE TABLE deleted_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,
    uid TEXT NOT NULL,
    deletion_token BIGINT NOT NULL DEFAULT 0,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for sync queries
CREATE INDEX idx_deleted_events_sync ON deleted_events(user_id, deleted_at);
CREATE INDEX idx_deleted_events_token ON deleted_events(user_id, deletion_token);

-- Trigger to capture deletions
CREATE OR REPLACE FUNCTION capture_event_deletion()
RETURNS TRIGGER AS $$
DECLARE
    current_sync_token BIGINT;
BEGIN
    -- Get current sync_token from users table
    SELECT COALESCE(NULLIF(sync_token, '')::bigint, 0)
    INTO current_sync_token
    FROM users
    WHERE telegram_id = OLD.user_id;

    INSERT INTO deleted_events (user_id, uid, deleted_at, deletion_token)
    VALUES (OLD.user_id, OLD.uid, NOW(), COALESCE(current_sync_token, 0));

    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER events_capture_deletion
    AFTER DELETE ON events
    FOR EACH ROW
    EXECUTE FUNCTION capture_event_deletion();

-- Comments
COMMENT ON TABLE deleted_events IS 'Tombstones for deleted events (for CalDAV sync)';
COMMENT ON COLUMN deleted_events.deletion_token IS 'Sync token at time of deletion';

-- Create event_attendees table
CREATE TABLE event_attendees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    telegram_id BIGINT,
    role TEXT NOT NULL DEFAULT 'ATTENDEE',
    status TEXT NOT NULL DEFAULT 'NEEDS-ACTION',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for event lookups
CREATE INDEX idx_event_attendees_event ON event_attendees(event_id);
CREATE INDEX idx_event_attendees_telegram ON event_attendees(telegram_id) WHERE telegram_id IS NOT NULL;

-- Unique constraint: one entry per email per event
CREATE UNIQUE INDEX idx_event_attendees_unique ON event_attendees(event_id, email);

-- Update trigger for attendees
CREATE TRIGGER event_attendees_updated_at
    BEFORE UPDATE ON event_attendees
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE event_attendees IS 'Event attendees/participants';
COMMENT ON COLUMN event_attendees.telegram_id IS 'Telegram ID for internal users, NULL for external invites';
COMMENT ON COLUMN event_attendees.role IS 'Attendee role: ORGANIZER or ATTENDEE';
COMMENT ON COLUMN event_attendees.status IS 'RSVP status: NEEDS-ACTION, ACCEPTED, DECLINED, TENTATIVE';
