DROP INDEX IF EXISTS idx_transaction_cursor;
CREATE INDEX IF NOT EXISTS idx_transaction_occurred_created ON transaction (occurred_at DESC, created_at DESC);

DROP INDEX IF EXISTS idx_vendor_cursor;
DROP INDEX IF EXISTS idx_category_cursor;
DROP INDEX IF EXISTS idx_budget_period_cursor;
DROP INDEX IF EXISTS idx_budget_category_cursor;
DROP INDEX IF EXISTS idx_budget_cursor;
DROP INDEX IF EXISTS idx_account_cursor;
