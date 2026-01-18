-- Create calendars table
CREATE TABLE IF NOT EXISTS calendars (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#3b82f6',
    sync_token TEXT NOT NULL DEFAULT '0',
    ctag TEXT NOT NULL DEFAULT '0',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One calendar per user (hard constraint)
CREATE UNIQUE INDEX idx_calendars_user_id ON calendars(user_id);

-- Index for sync operations
CREATE INDEX idx_calendars_sync_token ON calendars(sync_token);

-- Comments for documentation
COMMENT ON TABLE calendars IS 'Calendar collections - one per user';
COMMENT ON COLUMN calendars.sync_token IS 'RFC 6578 sync-collection token (incremented on any change)';
COMMENT ON COLUMN calendars.ctag IS 'CalDAV collection tag (timestamp of last change)';
