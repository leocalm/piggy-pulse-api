CREATE TABLE IF NOT EXISTS user_session
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_user_session_user_id ON user_session (user_id);
CREATE INDEX IF NOT EXISTS idx_user_session_expires_at ON user_session (expires_at);

ALTER TABLE account
    DROP CONSTRAINT IF EXISTS account_name_key;
ALTER TABLE category
    DROP CONSTRAINT IF EXISTS category_name_key;
ALTER TABLE vendor
    DROP CONSTRAINT IF EXISTS vendor_name_key;
ALTER TABLE budget_period
    DROP CONSTRAINT IF EXISTS budget_period_name_key;

ALTER TABLE account
    ADD CONSTRAINT account_user_id_name_key UNIQUE (user_id, name);
ALTER TABLE category
    ADD CONSTRAINT category_user_id_name_key UNIQUE (user_id, name);
ALTER TABLE vendor
    ADD CONSTRAINT vendor_user_id_name_key UNIQUE (user_id, name);
ALTER TABLE budget_period
    ADD CONSTRAINT budget_period_user_id_name_key UNIQUE (user_id, name);
