-- Add password_resets table for secure token-based password recovery
CREATE TABLE password_resets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(64) NOT NULL, -- SHA-256 hash of the reset token
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used_at TIMESTAMP WITH TIME ZONE,
    CONSTRAINT password_resets_unique_token UNIQUE (token_hash)
);

-- Indexes for efficient lookups
CREATE INDEX idx_password_resets_user_id ON password_resets(user_id);
CREATE INDEX idx_password_resets_token_hash ON password_resets(token_hash);
CREATE INDEX idx_password_resets_expires_at ON password_resets(expires_at);

-- Add security audit log for tracking password reset attempts and other security events
CREATE TABLE security_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    event_type VARCHAR(50) NOT NULL, -- e.g., 'password_reset_requested', 'password_reset_completed', 'password_reset_failed'
    ip_address INET,
    user_agent TEXT,
    success BOOLEAN NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for audit log queries
CREATE INDEX idx_security_audit_log_user_id ON security_audit_log(user_id);
CREATE INDEX idx_security_audit_log_event_type ON security_audit_log(event_type);
CREATE INDEX idx_security_audit_log_created_at ON security_audit_log(created_at);
CREATE INDEX idx_security_audit_log_success ON security_audit_log(success);

-- Function to automatically clean up expired password reset tokens
-- This can be called periodically or triggered by the application
CREATE OR REPLACE FUNCTION cleanup_expired_password_resets()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM password_resets
    WHERE expires_at < NOW()
    AND used_at IS NULL;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Add comment documentation
COMMENT ON TABLE password_resets IS 'Stores password reset tokens with expiration and usage tracking';
COMMENT ON TABLE security_audit_log IS 'Audit trail for security-related events including password resets';
COMMENT ON COLUMN password_resets.token_hash IS 'SHA-256 hash of the reset token for secure storage';
COMMENT ON COLUMN password_resets.used_at IS 'Timestamp when token was used; NULL means unused';
COMMENT ON FUNCTION cleanup_expired_password_resets() IS 'Removes expired password reset tokens from the database';
