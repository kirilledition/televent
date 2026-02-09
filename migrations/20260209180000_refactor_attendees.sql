-- =============================================================================
-- Refactor Attendees and Status
-- =============================================================================
-- 1. Rename participation_status to attendee_status
-- 2. Refactor event_attendees table:
--    - Rename telegram_id -> user_id
--    - Remove surrogate PK (id)
--    - Set composite PK (event_id, email)
--    - Remove redundant index
-- =============================================================================

-- 1. Rename Enum
ALTER TYPE participation_status RENAME TO attendee_status;

-- 2. Refactor event_attendees
-- Rename column
ALTER TABLE event_attendees
    RENAME COLUMN telegram_id TO user_id;

-- Drop constraints and columns
ALTER TABLE event_attendees
    DROP CONSTRAINT event_attendees_pkey,
    DROP COLUMN id;

-- Add new PK
ALTER TABLE event_attendees
    ADD PRIMARY KEY (event_id, email);

-- Drop redundant index (covered by PK)
DROP INDEX IF EXISTS idx_event_attendees_unique;

-- Comments update
COMMENT ON TABLE event_attendees IS 'Event attendees/participants';
COMMENT ON COLUMN event_attendees.user_id IS 'User ID (was telegram_id) for internal users, NULL for external invites';
COMMENT ON COLUMN event_attendees.status IS 'Attendee status (NEEDS-ACTION, ACCEPTED, DECLINED, TENTATIVE)';
