-- Add is_archived and description columns to category table
ALTER TABLE category
    ADD COLUMN is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN description TEXT NULL;

-- Add index for filtering by archived status
CREATE INDEX IF NOT EXISTS idx_category_archived ON category (user_id, is_archived);
