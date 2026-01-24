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
