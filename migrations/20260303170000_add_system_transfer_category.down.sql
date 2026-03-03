DROP INDEX IF EXISTS idx_category_system_transfer;
ALTER TABLE category DROP COLUMN IF EXISTS is_system;
