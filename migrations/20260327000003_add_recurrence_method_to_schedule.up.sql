ALTER TABLE period_schedule
    ADD COLUMN recurrence_method VARCHAR NOT NULL DEFAULT 'dayOfMonth';
