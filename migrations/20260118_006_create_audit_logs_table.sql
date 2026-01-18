-- Create audit_logs table for GDPR compliance
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Primary query: audit trail for a user
CREATE INDEX idx_audit_logs_user_time ON audit_logs(user_id, created_at DESC);

-- Index for entity-based lookups
CREATE INDEX idx_audit_logs_entity ON audit_logs(entity_type, entity_id);

-- Comments for documentation
COMMENT ON TABLE audit_logs IS 'Audit trail for GDPR compliance and security monitoring';
COMMENT ON COLUMN audit_logs.action IS 'Action performed: "event_created", "data_exported", "account_deleted", etc.';
COMMENT ON COLUMN audit_logs.entity_type IS 'Type of entity affected: "event", "calendar", "user"';
COMMENT ON COLUMN audit_logs.entity_id IS 'ID of the affected entity (if applicable)';
