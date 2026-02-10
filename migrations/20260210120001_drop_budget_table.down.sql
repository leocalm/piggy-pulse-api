-- Restore budget table
CREATE TABLE IF NOT EXISTS budget
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    name       TEXT        NOT NULL,
    start_day  INTEGER     NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Restore index
CREATE INDEX IF NOT EXISTS idx_budget_cursor ON budget (user_id, created_at DESC, id DESC);
