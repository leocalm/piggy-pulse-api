ALTER TABLE settings
    ADD COLUMN timezone    TEXT    NOT NULL DEFAULT 'UTC',
    ADD COLUMN date_format TEXT    NOT NULL DEFAULT 'DD/MM/YYYY',
    ADD COLUMN number_format TEXT  NOT NULL DEFAULT '1,234.56',
    ADD COLUMN compact_mode BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN period_mode  TEXT    NOT NULL DEFAULT 'manual'
        CHECK (period_mode IN ('automatic', 'manual'));
