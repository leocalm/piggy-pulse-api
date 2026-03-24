-- Allow transactions to be stored without a category (for the uncategorized feature).
-- Previously category_id was NOT NULL; making it nullable enables tracking transactions
-- that have not yet been assigned a category.
ALTER TABLE transaction ALTER COLUMN category_id DROP NOT NULL;
