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
