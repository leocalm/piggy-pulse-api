-- Drop overlay-related tables (in reverse order of dependencies)
DROP INDEX IF EXISTS idx_overlay_tx_inclusions_tx;
DROP INDEX IF EXISTS idx_overlay_tx_inclusions_overlay;
DROP TABLE IF EXISTS overlay_transaction_inclusions;

DROP INDEX IF EXISTS idx_overlay_category_caps_overlay;
DROP TABLE IF EXISTS overlay_category_caps;

DROP INDEX IF EXISTS idx_overlays_created_at;
DROP INDEX IF EXISTS idx_overlays_user_dates;
DROP INDEX IF EXISTS idx_overlays_user;
DROP TABLE IF EXISTS overlays;

-- Drop period_schedule table
DROP INDEX IF EXISTS idx_period_schedule_user;
DROP TABLE IF EXISTS period_schedule;

-- Remove is_auto_generated column from budget_period
ALTER TABLE budget_period DROP COLUMN IF EXISTS is_auto_generated;
