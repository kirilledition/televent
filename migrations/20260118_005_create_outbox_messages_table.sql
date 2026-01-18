-- Create outbox_status enum
CREATE TYPE outbox_status AS ENUM ('pending', 'processing', 'completed', 'failed');

-- Create outbox_messages table for async processing
CREATE TABLE IF NOT EXISTS outbox_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status outbox_status NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Primary query: pending jobs ordered by scheduled time
CREATE INDEX idx_outbox_pending ON outbox_messages(status, scheduled_at) 
    WHERE status = 'pending';

-- Index for monitoring and debugging
CREATE INDEX idx_outbox_created ON outbox_messages(created_at DESC);

-- Comments for documentation
COMMENT ON TABLE outbox_messages IS 'Outbox pattern for async email and notification processing';
COMMENT ON COLUMN outbox_messages.message_type IS 'Type discriminator: "email", "telegram_notification"';
COMMENT ON COLUMN outbox_messages.payload IS 'JSON payload with message-specific data';
COMMENT ON COLUMN outbox_messages.retry_count IS 'Number of retry attempts (max 5)';
COMMENT ON COLUMN outbox_messages.scheduled_at IS 'When to process this message (for delayed delivery)';
