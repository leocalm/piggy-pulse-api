-- Revert: restore the NOT NULL constraint on transaction.category_id.
-- Any rows with category_id IS NULL must be handled before running this revert.
ALTER TABLE transaction ALTER COLUMN category_id SET NOT NULL;
