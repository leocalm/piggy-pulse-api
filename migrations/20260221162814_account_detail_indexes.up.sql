-- Supports balance-history, detail, context, and transactions queries
-- All four endpoints query transactions by account and date
CREATE INDEX IF NOT EXISTS idx_transaction_from_account_date
    ON transaction (from_account_id, occurred_at);

CREATE INDEX IF NOT EXISTS idx_transaction_to_account_date
    ON transaction (to_account_id, occurred_at);
