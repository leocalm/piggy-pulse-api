-- Allow 'system' as a theme value (v2 API uses light/dark/system instead of auto)
ALTER TABLE settings DROP CONSTRAINT IF EXISTS settings_theme_check;
ALTER TABLE settings ADD CONSTRAINT settings_theme_check CHECK (theme IN ('light', 'dark', 'auto', 'system'));
