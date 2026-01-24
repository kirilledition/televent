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
