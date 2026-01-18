-- Create event_status enum
CREATE TYPE event_status AS ENUM ('CONFIRMED', 'TENTATIVE', 'CANCELLED');

-- Create events table
CREATE TABLE IF NOT EXISTS events (
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
    CONSTRAINT events_uid_unique UNIQUE (calendar_id, uid),
    CONSTRAINT events_end_after_start CHECK ("end" >= start)
);

-- Primary query pattern: events for a calendar in a date range
CREATE INDEX idx_events_calendar_start ON events(calendar_id, start);

-- Secondary indexes for CalDAV operations
CREATE INDEX idx_events_uid ON events(uid);
CREATE INDEX idx_events_calendar_updated ON events(calendar_id, updated_at);

-- Index for recurring events
CREATE INDEX idx_events_rrule ON events(calendar_id) WHERE rrule IS NOT NULL;

-- Comments for documentation
COMMENT ON TABLE events IS 'Calendar events with CalDAV/iCalendar properties';
COMMENT ON COLUMN events.uid IS 'iCalendar UID (stable identifier across syncs)';
COMMENT ON COLUMN events.rrule IS 'RFC 5545 recurrence rule (RRULE property)';
COMMENT ON COLUMN events.version IS 'Optimistic locking version (incremented on updates)';
COMMENT ON COLUMN events.etag IS 'HTTP ETag for conflict detection (SHA256 of event data)';
COMMENT ON COLUMN events.timezone IS 'VTIMEZONE reference for the event';
