-- Remove is_archived and description columns from category table
DROP INDEX IF EXISTS idx_category_archived;
ALTER TABLE category
    DROP COLUMN description,
    DROP COLUMN is_archived;
