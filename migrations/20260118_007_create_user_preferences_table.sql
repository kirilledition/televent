-- Create user_preferences table for notification settings
CREATE TABLE IF NOT EXISTS user_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    reminder_minutes_before INTEGER NOT NULL DEFAULT 15,
    daily_digest_time TIME NOT NULL DEFAULT '08:00:00',
    notifications_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT reminder_minutes_positive CHECK (reminder_minutes_before >= 0)
);

-- Comments for documentation
COMMENT ON TABLE user_preferences IS 'User notification and reminder preferences';
COMMENT ON COLUMN user_preferences.reminder_minutes_before IS 'Minutes before event to send reminder (0 = disabled)';
COMMENT ON COLUMN user_preferences.daily_digest_time IS 'Time of day to send daily digest (user timezone)';
