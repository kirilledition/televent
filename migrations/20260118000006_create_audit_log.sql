-- Create audit_log table
-- For GDPR compliance and security monitoring

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Performance index for user audit queries
CREATE INDEX idx_audit_user_time ON audit_log(user_id, created_at DESC);

-- Index for action-based queries
CREATE INDEX idx_audit_action ON audit_log(action, created_at DESC);

-- Partition by month for efficient archival (future optimization)
-- CREATE TABLE audit_log_y2026m01 PARTITION OF audit_log
--     FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');

-- Comments
COMMENT ON TABLE audit_log IS 'Audit log for GDPR compliance and security (2-year retention)';
COMMENT ON COLUMN audit_log.action IS 'Action performed: event_created, data_exported, account_deleted, etc.';
COMMENT ON COLUMN audit_log.entity_type IS 'Type of entity: event, calendar, user, etc.';
COMMENT ON COLUMN audit_log.entity_id IS 'UUID of affected entity';
COMMENT ON COLUMN audit_log.ip_address IS 'Client IP address (for security monitoring)';
