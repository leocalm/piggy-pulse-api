-- Indexes backing cursor-based pagination queries.
-- Each covers the (ORDER BY, PK) columns used in the WHERE (col, id) < (subquery) pattern.

CREATE INDEX IF NOT EXISTS idx_account_cursor        ON account        (created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_budget_cursor         ON budget         (created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_budget_category_cursor ON budget_category (created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_budget_period_cursor  ON budget_period  (start_date  ASC,  id ASC);
CREATE INDEX IF NOT EXISTS idx_category_cursor       ON category       (created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_vendor_cursor         ON vendor         (created_at DESC, id DESC);

-- Transaction cursor uses three columns; drop the two-column index it supersedes.
DROP INDEX IF EXISTS idx_transaction_occurred_created;
CREATE INDEX IF NOT EXISTS idx_transaction_cursor    ON transaction    (occurred_at DESC, created_at DESC, id DESC);
