-- Create user_preferences table
-- For notification and reminder settings

CREATE TABLE user_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    reminder_minutes_before INTEGER NOT NULL DEFAULT 15,
    daily_digest_time TIME NOT NULL DEFAULT '08:00:00',
    notifications_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_reminder_time CHECK (reminder_minutes_before >= 0 AND reminder_minutes_before <= 1440)
);

-- Update updated_at trigger
CREATE TRIGGER user_preferences_updated_at
    BEFORE UPDATE ON user_preferences
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE user_preferences IS 'User notification and reminder preferences';
COMMENT ON COLUMN user_preferences.reminder_minutes_before IS 'Minutes before event to send reminder (0-1440)';
COMMENT ON COLUMN user_preferences.daily_digest_time IS 'Time of day to send daily digest (in user timezone)';
COMMENT ON COLUMN user_preferences.notifications_enabled IS 'Master switch for all notifications';
