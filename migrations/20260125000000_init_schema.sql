-- Create users table
-- Stores Telegram user information and preferences

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    telegram_id BIGINT NOT NULL UNIQUE,
    telegram_username TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fast lookup by telegram_id
CREATE UNIQUE INDEX idx_users_telegram_id ON users(telegram_id);

-- Comments for documentation
COMMENT ON TABLE users IS 'Telegram users who have registered with Televent';
COMMENT ON COLUMN users.telegram_id IS 'Telegram user ID (unique across Telegram)';
COMMENT ON COLUMN users.timezone IS 'IANA timezone string (e.g., Asia/Singapore, Europe/London)';
-- Create calendars table
-- One calendar per user (hard constraint)

CREATE TABLE calendars (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL DEFAULT 'My Calendar',
    color TEXT NOT NULL DEFAULT '#3b82f6',
    sync_token TEXT NOT NULL DEFAULT '0',
    ctag TEXT NOT NULL DEFAULT '0',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Enforce one calendar per user
CREATE UNIQUE INDEX idx_calendars_user_id ON calendars(user_id);

-- Update updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER calendars_updated_at
    BEFORE UPDATE ON calendars
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE calendars IS 'User calendars (one per user)';
COMMENT ON COLUMN calendars.sync_token IS 'RFC 6578 sync token for CalDAV sync-collection';
COMMENT ON COLUMN calendars.ctag IS 'Collection tag for change detection (timestamp-based)';
COMMENT ON COLUMN calendars.color IS 'Hex color code for UI display';
-- Create event_status enum
CREATE TYPE event_status AS ENUM ('CONFIRMED', 'TENTATIVE', 'CANCELLED');

-- Create events table
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    calendar_id UUID NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
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
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_time_range CHECK ("end" > start)
);

-- Unique UID per calendar (UID is stable across CalDAV syncs)
CREATE UNIQUE INDEX idx_events_calendar_uid ON events(calendar_id, uid);

-- Performance index for time-based queries
CREATE INDEX idx_events_calendar_start ON events(calendar_id, start);
CREATE INDEX idx_events_calendar_updated ON events(calendar_id, updated_at);

-- Index for UID lookups
CREATE INDEX idx_events_uid ON events(uid);

-- Update updated_at trigger
CREATE TRIGGER events_updated_at
    BEFORE UPDATE ON events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE events IS 'Calendar events with iCalendar compatibility';
COMMENT ON COLUMN events.uid IS 'iCalendar UID (stable identifier across syncs)';
COMMENT ON COLUMN events.rrule IS 'RFC 5545 recurrence rule (RRULE)';
COMMENT ON COLUMN events.timezone IS 'VTIMEZONE reference for event times';
COMMENT ON COLUMN events.version IS 'Optimistic locking version number';
COMMENT ON COLUMN events.etag IS 'HTTP ETag (SHA256 of serialized event)';
COMMENT ON COLUMN events.status IS 'Event status: CONFIRMED, TENTATIVE, or CANCELLED';
-- Create device_passwords table
-- For CalDAV HTTP Basic Auth

CREATE TABLE device_passwords (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hashed_password TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

-- Index for user's devices
CREATE INDEX idx_device_passwords_user_id ON device_passwords(user_id);

-- Comments
COMMENT ON TABLE device_passwords IS 'Device-specific passwords for CalDAV authentication';
COMMENT ON COLUMN device_passwords.hashed_password IS 'Argon2id hashed password';
COMMENT ON COLUMN device_passwords.name IS 'User-friendly device label (e.g., iPhone, Thunderbird)';
COMMENT ON COLUMN device_passwords.last_used_at IS 'Last successful authentication timestamp';
-- Create outbox_status enum
CREATE TYPE outbox_status AS ENUM ('pending', 'processing', 'completed', 'failed');

-- Create outbox_messages table
-- Transactional outbox pattern for reliable messaging

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

-- Performance index for worker polling
CREATE INDEX idx_outbox_pending ON outbox_messages(status, scheduled_at)
    WHERE status = 'pending';

-- Index for monitoring failed messages
CREATE INDEX idx_outbox_failed ON outbox_messages(status, created_at)
    WHERE status = 'failed';

-- Comments
COMMENT ON TABLE outbox_messages IS 'Transactional outbox for async message processing';
COMMENT ON COLUMN outbox_messages.message_type IS 'Message type: email, telegram_notification, etc.';
COMMENT ON COLUMN outbox_messages.payload IS 'JSON payload with message-specific data';
COMMENT ON COLUMN outbox_messages.retry_count IS 'Number of retry attempts';
COMMENT ON COLUMN outbox_messages.scheduled_at IS 'When the message should be processed';
-- Create audit_log table
-- For GDPR compliance and security monitoring

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Performance index for user audit queries
CREATE INDEX idx_audit_user_time ON audit_log(user_id, created_at DESC);

-- Index for action-based queries
CREATE INDEX idx_audit_action ON audit_log(action, created_at DESC);

-- Partition by month for efficient archival (future optimization)
-- CREATE TABLE audit_log_y2026m01 PARTITION OF audit_log
--     FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');

-- Comments
COMMENT ON TABLE audit_log IS 'Audit log for GDPR compliance and security (2-year retention)';
COMMENT ON COLUMN audit_log.action IS 'Action performed: event_created, data_exported, account_deleted, etc.';
COMMENT ON COLUMN audit_log.entity_type IS 'Type of entity: event, calendar, user, etc.';
COMMENT ON COLUMN audit_log.entity_id IS 'UUID of affected entity';
COMMENT ON COLUMN audit_log.ip_address IS 'Client IP address (for security monitoring)';
-- Create user_preferences table
-- For notification and reminder settings

CREATE TABLE user_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    reminder_minutes_before INTEGER NOT NULL DEFAULT 15,
    daily_digest_time TIME NOT NULL DEFAULT '08:00:00',
    notifications_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_reminder_time CHECK (reminder_minutes_before >= 0 AND reminder_minutes_before <= 1440)
);

-- Update updated_at trigger
CREATE TRIGGER user_preferences_updated_at
    BEFORE UPDATE ON user_preferences
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE user_preferences IS 'User notification and reminder preferences';
COMMENT ON COLUMN user_preferences.reminder_minutes_before IS 'Minutes before event to send reminder (0-1440)';
COMMENT ON COLUMN user_preferences.daily_digest_time IS 'Time of day to send daily digest (in user timezone)';
COMMENT ON COLUMN user_preferences.notifications_enabled IS 'Master switch for all notifications';
-- Create deleted_users table
-- For GDPR compliance: 30-day grace period before permanent deletion

CREATE TABLE deleted_users (
    id UUID PRIMARY KEY,
    telegram_id BIGINT NOT NULL,
    telegram_username TEXT,
    data_snapshot JSONB NOT NULL,
    deletion_requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    permanent_deletion_at TIMESTAMPTZ NOT NULL,
    deleted_by TEXT DEFAULT 'user_request'
);

-- Index for finding users ready for permanent deletion
-- Note: We index all rows, then filter with WHERE clause in queries
CREATE INDEX idx_deleted_users_purge ON deleted_users(permanent_deletion_at);

-- Index for finding users by telegram_id (for recovery)
CREATE INDEX idx_deleted_users_telegram_id ON deleted_users(telegram_id);

-- Comments
COMMENT ON TABLE deleted_users IS 'Soft-deleted users with 30-day grace period for GDPR compliance';
COMMENT ON COLUMN deleted_users.data_snapshot IS 'Encrypted JSON snapshot of user data for recovery';
COMMENT ON COLUMN deleted_users.permanent_deletion_at IS 'When the user will be permanently deleted (30 days from request)';
COMMENT ON COLUMN deleted_users.deleted_by IS 'Deletion reason: user_request, admin_action, etc.';
-- Create audit triggers for automatic logging
-- Tracks event changes for GDPR compliance

-- Function to log event changes
CREATE OR REPLACE FUNCTION log_event_change()
RETURNS TRIGGER AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        SELECT c.user_id, 'event_created', 'event', NEW.id
        FROM calendars c
        WHERE c.id = NEW.calendar_id;
        RETURN NEW;
    ELSIF (TG_OP = 'UPDATE') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        SELECT c.user_id, 'event_updated', 'event', NEW.id
        FROM calendars c
        WHERE c.id = NEW.calendar_id;
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        SELECT c.user_id, 'event_deleted', 'event', OLD.id
        FROM calendars c
        WHERE c.id = OLD.calendar_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger for event changes
CREATE TRIGGER events_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON events
    FOR EACH ROW
    EXECUTE FUNCTION log_event_change();

-- Function to log calendar changes
CREATE OR REPLACE FUNCTION log_calendar_change()
RETURNS TRIGGER AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        VALUES (NEW.user_id, 'calendar_created', 'calendar', NEW.id);
        RETURN NEW;
    ELSIF (TG_OP = 'UPDATE') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        VALUES (NEW.user_id, 'calendar_updated', 'calendar', NEW.id);
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        INSERT INTO audit_log (user_id, action, entity_type, entity_id)
        VALUES (OLD.user_id, 'calendar_deleted', 'calendar', OLD.id);
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger for calendar changes
CREATE TRIGGER calendars_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON calendars
    FOR EACH ROW
    EXECUTE FUNCTION log_calendar_change();

-- Comments
COMMENT ON FUNCTION log_event_change() IS 'Automatically logs event changes to audit_log for GDPR compliance';
COMMENT ON FUNCTION log_calendar_change() IS 'Automatically logs calendar changes to audit_log for GDPR compliance';
-- Add index for outbox monitoring (requested in plan)
-- Provides efficient querying of message history by status
CREATE INDEX IF NOT EXISTS idx_outbox_status_created ON outbox_messages(status, created_at);
-- Create participation_status enum (following RFC 5545 PARTSTAT values)
CREATE TYPE participation_status AS ENUM (
    'NEEDS-ACTION',   -- Invitation pending (default)
    'ACCEPTED',       -- User accepted
    'DECLINED',       -- User declined
    'TENTATIVE'       -- User tentatively accepted
);

-- Create attendee_role enum (following RFC 5545 ROLE values)
CREATE TYPE attendee_role AS ENUM (
    'ORGANIZER',      -- Event creator
    'ATTENDEE'        -- Invited participant
);

-- Create event_attendees table
CREATE TABLE event_attendees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    email TEXT NOT NULL,              -- Can be: tg_123@televent.internal or real email
    telegram_id BIGINT,                -- NULL for external emails, populated for internal
    role attendee_role NOT NULL DEFAULT 'ATTENDEE',
    status participation_status NOT NULL DEFAULT 'NEEDS-ACTION',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Uniqueness: one RSVP per email per event
    CONSTRAINT unique_email_per_event UNIQUE (event_id, email)
);

-- Index for finding attendees by event
CREATE INDEX idx_attendees_event ON event_attendees(event_id);

-- Index for finding events by telegram_id (for /rsvp lookup)
CREATE INDEX idx_attendees_telegram_id ON event_attendees(telegram_id)
    WHERE telegram_id IS NOT NULL;

-- Index for finding pending invites for a user
CREATE INDEX idx_attendees_pending ON event_attendees(telegram_id, status)
    WHERE status = 'NEEDS-ACTION' AND telegram_id IS NOT NULL;

-- Update updated_at trigger
CREATE TRIGGER event_attendees_updated_at
    BEFORE UPDATE ON event_attendees
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE event_attendees IS 'Event attendees and their RSVP status';
COMMENT ON COLUMN event_attendees.email IS 'Email address: tg_<telegram_id>@televent.internal for internal users';
COMMENT ON COLUMN event_attendees.telegram_id IS 'Populated for internal users, NULL for external invites';
COMMENT ON COLUMN event_attendees.role IS 'Attendee role: ORGANIZER or ATTENDEE';
COMMENT ON COLUMN event_attendees.status IS 'RSVP status: NEEDS-ACTION, ACCEPTED, DECLINED, TENTATIVE';
-- Create deleted_events table for CalDAV sync
CREATE TABLE deleted_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    calendar_id UUID NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    uid TEXT NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for sync queries
CREATE INDEX idx_deleted_events_sync ON deleted_events(calendar_id, deleted_at);

-- Trigger to capture deletions
CREATE OR REPLACE FUNCTION capture_event_deletion()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO deleted_events (calendar_id, uid, deleted_at)
    VALUES (OLD.calendar_id, OLD.uid, NOW());
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER events_capture_deletion
    AFTER DELETE ON events
    FOR EACH ROW
    EXECUTE FUNCTION capture_event_deletion();
-- Add unique index for telegram_username to allow fast lookups by handle
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_telegram_username ON users(lower(telegram_username));
-- Add deletion_token to deleted_events
ALTER TABLE deleted_events ADD COLUMN deletion_token BIGINT NOT NULL DEFAULT 0;

-- Index for efficient querying by deletion_token
CREATE INDEX idx_deleted_events_token ON deleted_events(calendar_id, deletion_token);

-- Update trigger function to capture sync_token
CREATE OR REPLACE FUNCTION capture_event_deletion()
RETURNS TRIGGER AS $$
DECLARE
    current_sync_token BIGINT;
BEGIN
    -- Get current sync_token from calendars table
    -- Cast text to bigint, default to 0 if null or invalid
    SELECT COALESCE(NULLIF(sync_token, '')::bigint, 0)
    INTO current_sync_token
    FROM calendars
    WHERE id = OLD.calendar_id;

    INSERT INTO deleted_events (calendar_id, uid, deleted_at, deletion_token)
    VALUES (OLD.calendar_id, OLD.uid, NOW(), COALESCE(current_sync_token, 0));

    RETURN OLD;
END;
$$ LANGUAGE plpgsql;
