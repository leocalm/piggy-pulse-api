-- Add is_system column to category
ALTER TABLE category ADD COLUMN is_system BOOLEAN NOT NULL DEFAULT FALSE;

-- Create Transfer category for all existing users
INSERT INTO category (user_id, name, color, icon, category_type, is_system)
SELECT u.id, 'Transfer', '#868E96', '↔', 'Transfer'::category_type, TRUE
FROM users u
WHERE NOT EXISTS (
    SELECT 1 FROM category c
    WHERE c.user_id = u.id AND c.is_system = TRUE AND c.category_type = 'Transfer'::category_type
);

-- Backfill existing transfer transactions that have no category
UPDATE transaction t
SET category_id = c.id
FROM category c
WHERE c.user_id = t.user_id
  AND c.is_system = TRUE
  AND c.category_type = 'Transfer'::category_type
  AND t.to_account_id IS NOT NULL
  AND t.category_id IS NULL;

-- Prevent duplicate system Transfer categories per user
CREATE UNIQUE INDEX idx_category_system_transfer
  ON category (user_id) WHERE is_system = TRUE AND category_type = 'Transfer'::category_type;
