-- Create settings table for user preferences
CREATE TABLE IF NOT EXISTS settings (
    id                  UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id             UUID        NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE,
    theme               TEXT        NOT NULL DEFAULT 'light' CHECK (theme IN ('light', 'dark', 'auto')),
    language            TEXT        NOT NULL DEFAULT 'en' CHECK (language IN ('en', 'es', 'pt', 'fr', 'de')),
    default_currency_id UUID        NULL REFERENCES currency (id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create index for settings lookups by user_id
CREATE INDEX idx_settings_user ON settings (user_id);
