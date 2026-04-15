-- Revert immutable transaction ledger migration.
-- Reverses section 1-9 of the up migration in strict reverse order.

-- Drop redirected FKs so we can drop logical_transaction_state later
ALTER TABLE overlay_transaction_inclusions
    DROP CONSTRAINT IF EXISTS overlay_transaction_inclusions_transaction_id_fkey;
ALTER TABLE subscription_billing_event
    DROP CONSTRAINT IF EXISTS subscription_billing_event_transaction_id_fkey;

-- Drop aggregate maintenance trigger and function
DROP TRIGGER IF EXISTS transaction_aggregate_maintain ON transaction;
DROP FUNCTION IF EXISTS transaction_aggregate_maintenance();

-- Drop materialized aggregate tables
DROP TABLE IF EXISTS user_daily_totals;
DROP TABLE IF EXISTS vendor_category_all_time;
DROP TABLE IF EXISTS vendor_all_time;
DROP TABLE IF EXISTS vendor_daily_spend;
DROP TABLE IF EXISTS category_all_time;
DROP TABLE IF EXISTS category_daily_spend;
DROP TABLE IF EXISTS account_daily_delta;
DROP TABLE IF EXISTS account_balance_state;

-- Drop logical_transaction_state
DROP TABLE IF EXISTS logical_transaction_state;

-- Drop non-overlapping budget_period constraint
ALTER TABLE budget_period DROP CONSTRAINT IF EXISTS budget_period_no_overlap;

-- Drop type immutability triggers
DROP TRIGGER IF EXISTS category_type_immutable ON category;
DROP FUNCTION IF EXISTS reject_category_type_change();
DROP TRIGGER IF EXISTS account_type_immutable ON account;
DROP FUNCTION IF EXISTS reject_account_type_change();

-- Drop transaction immutability trigger
DROP TRIGGER IF EXISTS transaction_immutability ON transaction;
DROP FUNCTION IF EXISTS transaction_immutability_guard();

-- Revert transaction PK and amount type
ALTER TABLE transaction DROP CONSTRAINT transaction_pkey;
ALTER TABLE transaction DROP COLUMN seq;
ALTER TABLE transaction ADD CONSTRAINT transaction_pkey PRIMARY KEY (id);
ALTER TABLE transaction ALTER COLUMN amount TYPE INTEGER;

-- Restore original FKs pointing back at transaction(id)
ALTER TABLE overlay_transaction_inclusions
    ADD CONSTRAINT overlay_transaction_inclusions_transaction_id_fkey
        FOREIGN KEY (transaction_id) REFERENCES transaction (id) ON DELETE CASCADE;
ALTER TABLE subscription_billing_event
    ADD CONSTRAINT subscription_billing_event_transaction_id_fkey
        FOREIGN KEY (transaction_id) REFERENCES transaction (id) ON DELETE SET NULL;

-- Note: we intentionally do not DROP EXTENSION btree_gist in case other
-- migrations rely on it. It is safe to leave installed.
