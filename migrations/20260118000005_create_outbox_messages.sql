-- Create outbox_status enum
CREATE TYPE outbox_status AS ENUM ('pending', 'processing', 'completed', 'failed');

-- Create outbox_messages table
-- Transactional outbox pattern for reliable messaging

CREATE TABLE outbox_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status outbox_status NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_message TEXT
);

-- Performance index for worker polling
CREATE INDEX idx_outbox_pending ON outbox_messages(status, scheduled_at)
    WHERE status = 'pending';

-- Index for monitoring failed messages
CREATE INDEX idx_outbox_failed ON outbox_messages(status, created_at)
    WHERE status = 'failed';

-- Comments
COMMENT ON TABLE outbox_messages IS 'Transactional outbox for async message processing';
COMMENT ON COLUMN outbox_messages.message_type IS 'Message type: email, telegram_notification, etc.';
COMMENT ON COLUMN outbox_messages.payload IS 'JSON payload with message-specific data';
COMMENT ON COLUMN outbox_messages.retry_count IS 'Number of retry attempts';
COMMENT ON COLUMN outbox_messages.scheduled_at IS 'When the message should be processed';
