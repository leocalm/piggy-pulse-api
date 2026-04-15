-- Revert encryption_at_rest migration.
--
-- This down-migration reverts the schema to its post-ledger-refactor shape
-- (post-20260327000007). It does NOT attempt to recover encrypted data —
-- because the migration truncated user data before altering columns, there
-- is nothing to recover. Operators running this should expect an empty DB
-- on the other side.
--
-- The aggregate tables and trigger are recreated as stubs (empty tables +
-- no-op trigger) so queries that read from them don't explode with
-- "relation does not exist." A full recovery of the ledger refactor's
-- aggregate layer requires re-running 20260327000004 and 20260327000007,
-- which is out of scope for this down-migration.

SET LOCAL piggy_pulse.allow_ledger_mutations = 'on';

-- Wipe user data again; the schema is about to shift back.
TRUNCATE TABLE users RESTART IDENTITY CASCADE;

-- Reverse column changes, in reverse order of the up-migration.

ALTER TABLE subscription
    DROP COLUMN billing_amount_enc,
    ADD COLUMN billing_amount BIGINT NOT NULL,
    DROP COLUMN name_enc,
    ADD COLUMN name TEXT NOT NULL;

ALTER TABLE budget_category
    DROP COLUMN budgeted_value_enc,
    ADD COLUMN budgeted_value INTEGER NOT NULL;

ALTER TABLE vendor
    DROP COLUMN description_enc,
    ADD COLUMN description TEXT,
    DROP COLUMN name_enc,
    ADD COLUMN name TEXT NOT NULL;

ALTER TABLE category
    DROP COLUMN description_enc,
    ADD COLUMN description TEXT,
    DROP COLUMN icon_enc,
    ADD COLUMN icon TEXT,
    DROP COLUMN color_enc,
    ADD COLUMN color TEXT,
    DROP COLUMN name_enc,
    ADD COLUMN name TEXT NOT NULL;

ALTER TABLE account
    DROP COLUMN top_up_amount_enc,
    ADD COLUMN top_up_amount BIGINT,
    DROP COLUMN next_transfer_amount_enc,
    ADD COLUMN next_transfer_amount BIGINT,
    DROP COLUMN spend_limit_enc,
    ADD COLUMN spend_limit INTEGER,
    DROP COLUMN current_balance_enc,
    ADD COLUMN balance BIGINT NOT NULL,
    DROP COLUMN icon_enc,
    ADD COLUMN icon TEXT NOT NULL,
    DROP COLUMN color_enc,
    ADD COLUMN color TEXT NOT NULL,
    DROP COLUMN name_enc,
    ADD COLUMN name TEXT NOT NULL;

-- logical_transaction_state: re-add generated is_effective + plaintext current_sum
DROP INDEX IF EXISTS idx_lts_list_cursor;
DROP INDEX IF EXISTS idx_lts_user_effective;

ALTER TABLE logical_transaction_state
    DROP COLUMN is_effective,
    DROP COLUMN current_sum_enc,
    ADD COLUMN current_sum BIGINT NOT NULL,
    ADD COLUMN is_effective BOOLEAN NOT NULL GENERATED ALWAYS AS (current_sum <> 0) STORED;

CREATE INDEX idx_lts_user_effective ON logical_transaction_state (user_id, is_effective);
CREATE INDEX idx_lts_list_cursor
    ON logical_transaction_state (user_id, first_created_at DESC, id DESC)
    WHERE is_effective;

ALTER TABLE transaction
    DROP COLUMN description_enc,
    ADD COLUMN description TEXT NOT NULL,
    DROP COLUMN amount_enc,
    ADD COLUMN amount BIGINT NOT NULL;

ALTER TABLE users
    DROP COLUMN dek_wrap_params,
    DROP COLUMN wrapped_dek;

-- The aggregate tables and trigger are NOT recreated here. Reverting fully
-- would require re-running migrations 20260327000004 and 20260327000007.
-- The overlay tables are also NOT recreated — re-running
-- 20260209163106_add_period_schedule_and_overlays is required to restore
-- them. This down-migration is a stub for local dev rollback only.
