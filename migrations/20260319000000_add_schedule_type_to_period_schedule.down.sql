-- Revert: remove schedule_type column and restore NOT NULL constraints.
-- Delete manual schedules first since they have NULLs in auto fields.
DELETE FROM period_schedule WHERE schedule_type = 'manual';

ALTER TABLE period_schedule ALTER COLUMN start_day SET NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN duration_value SET NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN duration_unit SET NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN saturday_adjustment SET NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN sunday_adjustment SET NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN name_pattern SET NOT NULL;
ALTER TABLE period_schedule ALTER COLUMN generate_ahead SET NOT NULL;

ALTER TABLE period_schedule DROP COLUMN schedule_type;
