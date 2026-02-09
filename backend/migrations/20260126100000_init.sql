-- =============================================================================
-- Televent Database Schema - Initial Migration
-- =============================================================================
-- This migration establishes the complete database schema for Televent,
-- a Telegram bot with CalDAV sync capabilities for calendar management.
--
-- Schema Overview:
--   - Users table (user = calendar in CalDAV terms)
--   - Events table (supports both timed and all-day events)
--   - Event attendees (for multi-user event participation)
--   - Device passwords (for CalDAV authentication)
--   - Outbox messages (transactional outbox pattern)
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Custom Types (ENUMs)
-- -----------------------------------------------------------------------------

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

CREATE TYPE participation_status AS ENUM (
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

-- -----------------------------------------------------------------------------
-- 3. Users Table (User = Calendar)
-- -----------------------------------------------------------------------------

CREATE TABLE users (
    -- Identity
    telegram_id BIGINT PRIMARY KEY,
    telegram_username TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',

    -- Calendar Properties (CalDAV)
    sync_token TEXT NOT NULL DEFAULT '0',
    ctag TEXT NOT NULL DEFAULT '0',

    -- Metadata
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
    'RFC 6578 sync token for CalDAV sync-collection';
COMMENT ON COLUMN users.ctag IS
    'Collection tag for change detection (timestamp-based)';

-- -----------------------------------------------------------------------------
-- 4. Events Table
-- -----------------------------------------------------------------------------
-- Supports two event types:
--   1. Timed events: Use start/end (TIMESTAMPTZ)
--   2. All-day events: Use start_date/end_date (DATE)
-- The check constraint enforces XOR: exactly one type per event.
-- -----------------------------------------------------------------------------

CREATE TABLE events (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,
    uid TEXT NOT NULL,

    -- Content
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,

    -- Timing: Timed Events (mutually exclusive with all-day)
    start TIMESTAMPTZ,
    "end" TIMESTAMPTZ,

    -- Timing: All-Day Events (mutually exclusive with timed)
    start_date DATE,
    end_date DATE,
    is_all_day BOOLEAN NOT NULL DEFAULT FALSE,

    -- Properties
    status event_status NOT NULL DEFAULT 'CONFIRMED',
    rrule TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',

    -- Versioning & Sync
    version INTEGER NOT NULL DEFAULT 1,
    etag TEXT NOT NULL,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Integrity: XOR constraint - must be either timed OR all-day, never both
    CONSTRAINT check_event_type_integrity CHECK (
        (is_all_day = true AND
         start_date IS NOT NULL AND end_date IS NOT NULL AND
         start IS NULL AND "end" IS NULL)
        OR
        (is_all_day = false AND
         start_date IS NULL AND end_date IS NULL AND
         start IS NOT NULL AND "end" IS NOT NULL)
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
COMMENT ON COLUMN events.is_all_day IS
    'Distinguishes all-day events (DATE) from timed events (TIMESTAMPTZ)';
COMMENT ON CONSTRAINT check_event_type_integrity ON events IS
    'Ensures events are either timed (start/end) OR all-day (start_date/end_date), never both';

-- -----------------------------------------------------------------------------
-- 5. Event Attendees Table
-- -----------------------------------------------------------------------------
-- Tracks participants for events. Supports both:
--   - Internal users (via telegram_id)
--   - External invitees (email only, telegram_id = NULL)
-- -----------------------------------------------------------------------------

CREATE TABLE event_attendees (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,

    -- Attendee Identification
    email TEXT NOT NULL,
    telegram_id BIGINT,

    -- Participation
    role attendee_role NOT NULL DEFAULT 'ATTENDEE',
    status participation_status NOT NULL DEFAULT 'NEEDS-ACTION',

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_event_attendees_event
    ON event_attendees(event_id);

CREATE INDEX idx_event_attendees_telegram
    ON event_attendees(telegram_id)
    WHERE telegram_id IS NOT NULL;

CREATE UNIQUE INDEX idx_event_attendees_unique
    ON event_attendees(event_id, email);

-- Triggers
CREATE TRIGGER event_attendees_updated_at
    BEFORE UPDATE ON event_attendees
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Documentation
COMMENT ON TABLE event_attendees IS
    'Event attendees/participants';
COMMENT ON COLUMN event_attendees.telegram_id IS
    'Telegram ID for internal users, NULL for external invites';
COMMENT ON COLUMN event_attendees.email IS
    'Email address (required for CalDAV ATTENDEE property)';

-- -----------------------------------------------------------------------------
-- 6. Device Passwords Table
-- -----------------------------------------------------------------------------
-- Stores device-specific passwords for CalDAV authentication.
-- Users generate separate passwords for each device/app connecting via CalDAV.
-- -----------------------------------------------------------------------------

CREATE TABLE device_passwords (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL REFERENCES users(telegram_id) ON DELETE CASCADE,

    -- Device Info
    device_name TEXT NOT NULL,

    -- Authentication
    password_hash TEXT NOT NULL,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

-- Indexes
CREATE INDEX idx_device_passwords_user
    ON device_passwords(user_id);

-- Documentation
COMMENT ON TABLE device_passwords IS
    'Device-specific passwords for CalDAV authentication';
COMMENT ON COLUMN device_passwords.password_hash IS
    'Argon2id hash of device password';
COMMENT ON COLUMN device_passwords.last_used_at IS
    'Last successful authentication timestamp (for security auditing)';

-- -----------------------------------------------------------------------------
-- 7. Outbox Messages Table
-- -----------------------------------------------------------------------------
-- Implements the Transactional Outbox pattern for reliable async messaging.
-- Messages are written atomically with business transactions, then processed
-- asynchronously to ensure at-least-once delivery guarantees.
-- -----------------------------------------------------------------------------

CREATE TABLE outbox_messages (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Message
    message_type TEXT NOT NULL,
    payload JSONB NOT NULL,

    -- Processing State
    status outbox_status NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_message TEXT
);

-- Indexes (partial indexes for efficiency)
CREATE INDEX idx_outbox_pending
    ON outbox_messages(status, scheduled_at)
    WHERE status = 'pending';

CREATE INDEX idx_outbox_failed
    ON outbox_messages(status, created_at)
    WHERE status = 'failed';

-- Documentation
COMMENT ON TABLE outbox_messages IS
    'Transactional outbox for reliable async message processing';
COMMENT ON COLUMN outbox_messages.message_type IS
    'Message discriminator for routing to appropriate handlers';
COMMENT ON COLUMN outbox_messages.retry_count IS
    'Number of processing attempts (for exponential backoff)';

-- =============================================================================
-- End of Migration
-- =============================================================================
