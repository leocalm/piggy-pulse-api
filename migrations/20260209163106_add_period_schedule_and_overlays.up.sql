-- Add is_auto_generated column to budget_period
ALTER TABLE budget_period ADD COLUMN is_auto_generated BOOLEAN NOT NULL DEFAULT false;

-- Create period_schedule table for automatic period generation configuration
CREATE TABLE IF NOT EXISTS period_schedule (
    id                    UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id               UUID        NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE,
    start_day             INTEGER     NOT NULL CHECK (start_day >= 1 AND start_day <= 31),
    duration_value        INTEGER     NOT NULL CHECK (duration_value > 0),
    duration_unit         TEXT        NOT NULL CHECK (duration_unit IN ('days', 'weeks', 'months')),
    saturday_adjustment   TEXT        NOT NULL CHECK (saturday_adjustment IN ('keep', 'friday', 'monday')),
    sunday_adjustment     TEXT        NOT NULL CHECK (sunday_adjustment IN ('keep', 'friday', 'monday')),
    name_pattern          TEXT        NOT NULL,
    generate_ahead        INTEGER     NOT NULL CHECK (generate_ahead >= 0),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create index for period_schedule lookups
CREATE INDEX idx_period_schedule_user ON period_schedule (user_id);

-- Create overlays table for tracking spending across specific date ranges
CREATE TABLE IF NOT EXISTS overlays (
    id               UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id          UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name             TEXT        NOT NULL,
    icon             TEXT,
    start_date       DATE        NOT NULL,
    end_date         DATE        NOT NULL,
    inclusion_mode   TEXT        NOT NULL CHECK (inclusion_mode IN ('manual', 'rules', 'all')),
    total_cap_amount BIGINT,
    rules            JSONB       NOT NULL DEFAULT '{"category_ids": [], "vendor_ids": [], "account_ids": []}'::jsonb,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT overlays_date_range_check CHECK (end_date >= start_date)
);

-- Create indexes for overlays
CREATE INDEX idx_overlays_user ON overlays (user_id);
CREATE INDEX idx_overlays_user_dates ON overlays (user_id, start_date, end_date);
CREATE INDEX idx_overlays_created_at ON overlays (user_id, created_at DESC, id DESC);

-- Create overlay_category_caps table for category-specific spending caps within overlays
CREATE TABLE IF NOT EXISTS overlay_category_caps (
    id          UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    overlay_id  UUID        NOT NULL REFERENCES overlays (id) ON DELETE CASCADE,
    category_id UUID        NOT NULL REFERENCES category (id) ON DELETE CASCADE,
    cap_amount  BIGINT      NOT NULL CHECK (cap_amount >= 0),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (overlay_id, category_id)
);

-- Create indexes for overlay_category_caps
CREATE INDEX idx_overlay_category_caps_overlay ON overlay_category_caps (overlay_id);

-- Create overlay_transaction_inclusions table for manual transaction include/exclude
CREATE TABLE IF NOT EXISTS overlay_transaction_inclusions (
    id             UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    overlay_id     UUID        NOT NULL REFERENCES overlays (id) ON DELETE CASCADE,
    transaction_id UUID        NOT NULL REFERENCES transaction (id) ON DELETE CASCADE,
    is_included    BOOLEAN     NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (overlay_id, transaction_id)
);

-- Create indexes for overlay_transaction_inclusions
CREATE INDEX idx_overlay_tx_inclusions_overlay ON overlay_transaction_inclusions (overlay_id);
CREATE INDEX idx_overlay_tx_inclusions_tx ON overlay_transaction_inclusions (transaction_id);
