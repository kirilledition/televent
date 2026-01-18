-- Create device_passwords table
-- For CalDAV HTTP Basic Auth

CREATE TABLE device_passwords (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hashed_password TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

-- Index for user's devices
CREATE INDEX idx_device_passwords_user_id ON device_passwords(user_id);

-- Comments
COMMENT ON TABLE device_passwords IS 'Device-specific passwords for CalDAV authentication';
COMMENT ON COLUMN device_passwords.hashed_password IS 'Argon2id hashed password';
COMMENT ON COLUMN device_passwords.name IS 'User-friendly device label (e.g., iPhone, Thunderbird)';
COMMENT ON COLUMN device_passwords.last_used_at IS 'Last successful authentication timestamp';
