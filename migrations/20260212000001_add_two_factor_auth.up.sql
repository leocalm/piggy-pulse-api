-- Two-Factor Authentication Tables
-- This migration adds support for TOTP-based 2FA with backup codes and rate limiting

-- Main 2FA table: stores encrypted TOTP secrets
CREATE TABLE IF NOT EXISTS two_factor_auth
(
    id                UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id           UUID        NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE,
    encrypted_secret  TEXT        NOT NULL, -- AES-256-GCM encrypted TOTP secret (base64)
    encryption_nonce  TEXT        NOT NULL, -- GCM nonce for decryption (base64)
    is_enabled        BOOLEAN     NOT NULL DEFAULT false,
    verified_at       TIMESTAMPTZ NULL,     -- When user first successfully verified setup
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Backup recovery codes (one-time use, Argon2 hashed)
CREATE TABLE IF NOT EXISTS two_factor_backup_codes
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    code_hash  TEXT        NOT NULL, -- Argon2 hash of backup code
    used_at    TIMESTAMPTZ NULL,     -- NULL = unused, timestamp = when it was used
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Rate limiting for 2FA attempts (prevents brute force)
CREATE TABLE IF NOT EXISTS two_factor_rate_limits
(
    id               UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id          UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    failed_attempts  INTEGER     NOT NULL DEFAULT 0,
    locked_until     TIMESTAMPTZ NULL,     -- NULL = not locked, timestamp = locked until this time
    last_attempt_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Emergency disable tokens (email-based recovery)
CREATE TABLE IF NOT EXISTS two_factor_emergency_tokens
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL, -- SHA-256 hash of token
    expires_at TIMESTAMPTZ NOT NULL, -- 1 hour expiry
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    used_at    TIMESTAMPTZ NULL      -- NULL = unused
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_two_factor_auth_user_id ON two_factor_auth (user_id);
CREATE INDEX IF NOT EXISTS idx_backup_codes_user_id_used ON two_factor_backup_codes (user_id, used_at);
CREATE INDEX IF NOT EXISTS idx_rate_limit_user_id ON two_factor_rate_limits (user_id);
CREATE INDEX IF NOT EXISTS idx_rate_limit_locked ON two_factor_rate_limits (locked_until) WHERE locked_until IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_emergency_tokens_user_id ON two_factor_emergency_tokens (user_id);
CREATE INDEX IF NOT EXISTS idx_emergency_tokens_expires ON two_factor_emergency_tokens (expires_at);

-- Ensure only one active 2FA configuration per user (enforced by UNIQUE constraint on user_id)
-- Ensure only one rate limit record per user
CREATE UNIQUE INDEX IF NOT EXISTS idx_rate_limit_user_unique ON two_factor_rate_limits (user_id);
