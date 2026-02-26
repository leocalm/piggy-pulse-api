-- Table for tracking login attempts and enforcing rate limits
CREATE TABLE login_rate_limits (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identifier_type   VARCHAR(10) NOT NULL CHECK (identifier_type IN ('user_id', 'ip_address')),
    identifier_value  VARCHAR(255) NOT NULL,
    failed_attempts   INTEGER NOT NULL DEFAULT 0,
    last_attempt_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until      TIMESTAMPTZ NULL,
    next_attempt_at   TIMESTAMPTZ NULL,
    unlock_token      VARCHAR(255) NULL,
    unlock_token_expires_at TIMESTAMPTZ NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Unique constraint for identifier combination
CREATE UNIQUE INDEX idx_login_rate_limits_identifier
    ON login_rate_limits(identifier_type, identifier_value);

-- Index for locked accounts
CREATE INDEX idx_login_rate_limits_locked
    ON login_rate_limits(locked_until) WHERE locked_until IS NOT NULL;

-- Index for delayed attempts
CREATE INDEX idx_login_rate_limits_next_attempt
    ON login_rate_limits(next_attempt_at) WHERE next_attempt_at IS NOT NULL;

-- Index for unlock tokens
CREATE INDEX idx_login_rate_limits_unlock_token
    ON login_rate_limits(unlock_token) WHERE unlock_token IS NOT NULL;

-- Add new event types to audit log
COMMENT ON TABLE login_rate_limits IS 'Tracks failed login attempts and enforces rate limiting';