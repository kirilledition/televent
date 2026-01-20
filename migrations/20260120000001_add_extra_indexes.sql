-- Add index for outbox monitoring (requested in plan)
-- Provides efficient querying of message history by status
CREATE INDEX IF NOT EXISTS idx_outbox_status_created ON outbox_messages(status, created_at);
