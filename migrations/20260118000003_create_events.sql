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
