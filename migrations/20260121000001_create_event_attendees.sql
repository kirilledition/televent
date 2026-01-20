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
