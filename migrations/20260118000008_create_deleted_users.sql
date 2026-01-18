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
CREATE INDEX idx_deleted_users_purge ON deleted_users(permanent_deletion_at)
    WHERE permanent_deletion_at < NOW();

-- Index for finding users by telegram_id (for recovery)
CREATE INDEX idx_deleted_users_telegram_id ON deleted_users(telegram_id);

-- Comments
COMMENT ON TABLE deleted_users IS 'Soft-deleted users with 30-day grace period for GDPR compliance';
COMMENT ON COLUMN deleted_users.data_snapshot IS 'Encrypted JSON snapshot of user data for recovery';
COMMENT ON COLUMN deleted_users.permanent_deletion_at IS 'When the user will be permanently deleted (30 days from request)';
COMMENT ON COLUMN deleted_users.deleted_by IS 'Deletion reason: user_request, admin_action, etc.';
