-- Create users table
-- Stores Telegram user information and preferences

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    telegram_id BIGINT NOT NULL UNIQUE,
    telegram_username TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fast lookup by telegram_id
CREATE UNIQUE INDEX idx_users_telegram_id ON users(telegram_id);

-- Comments for documentation
COMMENT ON TABLE users IS 'Telegram users who have registered with Televent';
COMMENT ON COLUMN users.telegram_id IS 'Telegram user ID (unique across Telegram)';
COMMENT ON COLUMN users.timezone IS 'IANA timezone string (e.g., Asia/Singapore, Europe/London)';
