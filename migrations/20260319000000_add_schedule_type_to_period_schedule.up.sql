-- Add schedule_type column to period_schedule for V2 manual/automatic discrimination.
-- Existing rows are all automatic schedules.
ALTER TABLE period_schedule ADD COLUMN schedule_type TEXT NOT NULL DEFAULT 'automatic'
    CHECK (schedule_type IN ('manual', 'automatic'));

-- Make auto-generation fields nullable so manual schedules can omit them.
ALTER TABLE period_schedule ALTER COLUMN start_day DROP NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN duration_value DROP NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN duration_unit DROP NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN saturday_adjustment DROP NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN sunday_adjustment DROP NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN name_pattern DROP NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN generate_ahead DROP NOT NULL;
