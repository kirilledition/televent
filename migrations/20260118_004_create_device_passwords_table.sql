-- Create device_passwords table for CalDAV authentication
CREATE TABLE IF NOT EXISTS device_passwords (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hashed_password TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

-- Index for authentication lookups
CREATE INDEX idx_device_passwords_user_id ON device_passwords(user_id);

-- Comments for documentation
COMMENT ON TABLE device_passwords IS 'Device-specific passwords for CalDAV HTTP Basic Auth';
COMMENT ON COLUMN device_passwords.hashed_password IS 'Argon2id hash of the device password';
COMMENT ON COLUMN device_passwords.name IS 'User-friendly label (e.g., "iPhone", "Thunderbird")';
COMMENT ON COLUMN device_passwords.last_used_at IS 'Timestamp of last successful authentication';
