-- Add unique index for telegram_username to allow fast lookups by handle
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_telegram_username ON users(lower(telegram_username));
