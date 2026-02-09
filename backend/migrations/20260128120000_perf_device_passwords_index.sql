-- =============================================================================
-- Migration: Add index for device passwords lookup performance
-- =============================================================================
-- The CalDAV authentication middleware frequently queries device passwords
-- for a user, ordered by `last_used_at` (DESC NULLS LAST) and `created_at` (DESC).
--
-- This index allows the database to retrieve the top 10 results directly from
-- the index without performing a sort operation, significantly improving performance
-- for users with many devices or under high load.
-- =============================================================================

CREATE INDEX idx_device_passwords_auth_perf
    ON device_passwords(user_id, last_used_at DESC NULLS LAST, created_at DESC);

COMMENT ON INDEX idx_device_passwords_auth_perf IS
    'Optimizes device password lookup for CalDAV authentication by matching query sort order';
