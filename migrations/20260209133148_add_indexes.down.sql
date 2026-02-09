-- Rollback: drop all indexes added in 0002 and restore the original
-- single-column FK indexes on the transaction table.

-- Drop new indexes
DROP INDEX IF EXISTS idx_users_email;
DROP INDEX IF EXISTS idx_currency_code;
DROP INDEX IF EXISTS idx_account_currency_id;
DROP INDEX IF EXISTS idx_budget_category_category_id;
DROP INDEX IF EXISTS idx_budget_period_current;
DROP INDEX IF EXISTS idx_category_user_type;
DROP INDEX IF EXISTS idx_transaction_user_category;
DROP INDEX IF EXISTS idx_transaction_user_from_account;
DROP INDEX IF EXISTS idx_transaction_user_to_account;
DROP INDEX IF EXISTS idx_transaction_vendor_user;

-- Restore original single-column FK indexes
CREATE INDEX IF NOT EXISTS idx_transaction_category_id ON transaction (category_id);
CREATE INDEX IF NOT EXISTS idx_transaction_from_account_id ON transaction (from_account_id);
CREATE INDEX IF NOT EXISTS idx_transaction_to_account_id ON transaction (to_account_id);
CREATE INDEX IF NOT EXISTS idx_transaction_vendor_id ON transaction (vendor_id) WHERE vendor_id IS NOT NULL;
