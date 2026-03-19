-- Revert: remove 'system' from theme check constraint
ALTER TABLE settings DROP CONSTRAINT IF EXISTS settings_theme_check;
ALTER TABLE settings ADD CONSTRAINT settings_theme_check CHECK (theme IN ('light', 'dark', 'auto'));
