-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    telegram_id BIGINT NOT NULL UNIQUE,
    telegram_username TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on telegram_id for fast lookups during authentication
CREATE INDEX idx_users_telegram_id ON users(telegram_id);

-- Comments for documentation
COMMENT ON TABLE users IS 'Users authenticated via Telegram';
COMMENT ON COLUMN users.telegram_id IS 'Telegram user ID (unique identifier from Telegram API)';
COMMENT ON COLUMN users.timezone IS 'IANA timezone string (e.g., "Asia/Singapore", "America/New_York")';
