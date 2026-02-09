-- Add up migration script here
-- =============================================================================
-- 0002_add_indexes
--
-- Adds missing indexes for:
--   • Lookup columns used in WHERE clauses (users.email, currency.currency)
--   • Foreign-key columns that lack indexes (account.currency_id,
--     budget_category.category_id) — needed for JOIN performance and
--     ON DELETE CASCADE scans
--   • Composite (user_id, fk_col) indexes on the transaction table so that
--     user-scoped joins and aggregations can use an index-only prefix scan
--     instead of hitting the single-column FK index and then filtering
--   • A covering index on budget_period for the "current period" query
--   • A composite on category for the "not in budget" filter
--
-- The new composite transaction indexes replace the old single-column FK
-- indexes, which are dropped to reduce write amplification and storage.
-- =============================================================================

-- 1. users: unique index on email for login lookups + data integrity
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email
    ON users (email);

-- 2. currency: index on currency code for code-based lookups
CREATE INDEX IF NOT EXISTS idx_currency_code
    ON currency (currency);

-- 3. account: FK index on currency_id for joins and cascade deletes
CREATE INDEX IF NOT EXISTS idx_account_currency_id
    ON account (currency_id);

-- 4. budget_category: FK index on category_id for joins and cascade deletes
CREATE INDEX IF NOT EXISTS idx_budget_category_category_id
    ON budget_category (category_id);

-- 5. budget_period: composite for get_current_budget_period
--    WHERE user_id = $1 AND start_date <= now() AND end_date >= now()
CREATE INDEX IF NOT EXISTS idx_budget_period_current
    ON budget_period (user_id, start_date, end_date);

-- 6. category: composite for "not in budget" queries
--    WHERE user_id = $1 AND category_type = 'Outgoing'
CREATE INDEX IF NOT EXISTS idx_category_user_type
    ON category (user_id, category_type);

-- 7–10. transaction: replace single-column FK indexes with user-scoped composites
--
-- Drop the old single-column indexes first.
DROP INDEX IF EXISTS idx_transaction_category_id;
DROP INDEX IF EXISTS idx_transaction_from_account_id;
DROP INDEX IF EXISTS idx_transaction_to_account_id;
DROP INDEX IF EXISTS idx_transaction_vendor_id;

-- 7. transaction(user_id, category_id) — category aggregation in dashboard
--    and category list queries
CREATE INDEX IF NOT EXISTS idx_transaction_user_category
    ON transaction (user_id, category_id);

-- 8. transaction(user_id, from_account_id) — account balance computations
CREATE INDEX IF NOT EXISTS idx_transaction_user_from_account
    ON transaction (user_id, from_account_id);

-- 9. transaction(user_id, to_account_id) — account balance computations
CREATE INDEX IF NOT EXISTS idx_transaction_user_to_account
    ON transaction (user_id, to_account_id);

-- 10. transaction(vendor_id, user_id) — vendor stats queries that join on
--     vendor_id and filter by user_id; partial on vendor_id IS NOT NULL
CREATE INDEX IF NOT EXISTS idx_transaction_vendor_user
    ON transaction (vendor_id, user_id) WHERE vendor_id IS NOT NULL;
