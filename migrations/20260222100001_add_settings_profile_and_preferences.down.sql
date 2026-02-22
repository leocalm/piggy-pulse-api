ALTER TABLE settings
    DROP COLUMN IF EXISTS timezone,
    DROP COLUMN IF EXISTS date_format,
    DROP COLUMN IF EXISTS number_format,
    DROP COLUMN IF EXISTS compact_mode,
    DROP COLUMN IF EXISTS period_mode;
