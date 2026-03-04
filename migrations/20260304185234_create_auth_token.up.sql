CREATE TABLE api_tokens (
                            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                            access_token_hash VARCHAR(64) NOT NULL,
                            refresh_token_hash VARCHAR(64) NOT NULL,
                            device_name VARCHAR(255),
                            device_id VARCHAR(255),
                            expires_at TIMESTAMPTZ NOT NULL,
                            refresh_expires_at TIMESTAMPTZ NOT NULL,
                            last_used_at TIMESTAMPTZ,
                            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                            revoked_at TIMESTAMPTZ,

                            CONSTRAINT unique_device UNIQUE(user_id, device_id)
);

CREATE INDEX idx_api_tokens_access ON api_tokens(access_token_hash) WHERE revoked_at IS NULL;
CREATE INDEX idx_api_tokens_refresh ON api_tokens(refresh_token_hash) WHERE revoked_at IS NULL;
CREATE INDEX idx_api_tokens_user ON api_tokens(user_id);
