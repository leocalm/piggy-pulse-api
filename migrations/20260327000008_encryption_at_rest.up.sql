-- Encryption at Rest
--
-- See .kiro/specs/encryption-at-rest/design.md for the full design. This
-- migration is destructive: it replaces every plaintext column that holds
-- amounts, descriptions, names, or display metadata with an encrypted
-- counterpart, drops the materialized aggregate tables and their trigger
-- from the ledger refactor, and adds the per-user wrapped-DEK storage.
--
-- Because there are no existing users (see §"Constraints Already Confirmed"
-- in the design doc), this migration truncates every user-data table before
-- altering columns rather than attempting to encrypt plaintext in place.
-- Operators running this against a dev database with test data should expect
-- their fixtures to be wiped.

-- ───────────────────────────────────────────────────────────────────────
-- 1. Wipe user data so the column swaps below don't need backfill
-- ───────────────────────────────────────────────────────────────────────
-- The ledger immutability trigger blocks DELETE on `transaction`; allow it
-- for this one-shot destructive migration only. SET LOCAL is cleared on
-- commit, so the bypass doesn't leak.
SET LOCAL piggy_pulse.allow_ledger_mutations = 'on';

-- TRUNCATE users CASCADE removes every row in every table that has a FK
-- chain back to users. This is a table-level operation that ignores
-- ON DELETE RESTRICT (unlike row-level DELETE, where RESTRICT would block
-- us). TRUNCATE does not fire the BEFORE UPDATE OR DELETE immutability
-- trigger, but we keep the ledger mutation bypass set for safety in case
-- any subordinate cascade still routes through a DELETE trigger.
TRUNCATE TABLE users RESTART IDENTITY CASCADE;

-- ───────────────────────────────────────────────────────────────────────
-- 2. Drop the aggregate tables and trigger from the ledger refactor
-- ───────────────────────────────────────────────────────────────────────
-- The whole point of the ledger refactor's aggregate layer was to support
-- O(1) dashboard reads over plaintext amounts. Under encryption there's no
-- useful form for these tables — the trigger can't see plaintext, and the
-- read paths can't SUM over ciphertext. Everything that was computed here
-- is now derived client-side from the full-period fetch.

DROP TRIGGER IF EXISTS transaction_aggregate_maintain ON transaction;
DROP FUNCTION IF EXISTS transaction_aggregate_maintenance();

DROP TABLE IF EXISTS user_daily_totals;
DROP TABLE IF EXISTS vendor_category_all_time;
DROP TABLE IF EXISTS vendor_all_time;
DROP TABLE IF EXISTS vendor_daily_spend;
DROP TABLE IF EXISTS category_all_time;
DROP TABLE IF EXISTS category_daily_spend;
DROP TABLE IF EXISTS account_daily_delta;
DROP TABLE IF EXISTS account_balance_state;

-- ───────────────────────────────────────────────────────────────────────
-- 3. users: per-user wrapped DEK storage
-- ───────────────────────────────────────────────────────────────────────
-- wrapped_dek stores the 32-byte DEK encrypted with the user's KEK
-- (Argon2id-derived from their password). dek_wrap_params stores the
-- Argon2 cost parameters, salt, and AES-GCM wrap nonce as a JSON object
-- so parameter upgrades are possible without a schema change.
-- Both are NULL until the user completes the unlock flow for the first
-- time; after signup + first login the user's client has materialized a
-- DEK and written the wrapped form here.

ALTER TABLE users
    ADD COLUMN wrapped_dek      BYTEA,
    ADD COLUMN dek_wrap_params  JSONB;

-- ───────────────────────────────────────────────────────────────────────
-- 4. transaction: encrypt amount + description
-- ───────────────────────────────────────────────────────────────────────
ALTER TABLE transaction
    DROP COLUMN amount,
    ADD COLUMN amount_enc BYTEA NOT NULL,
    DROP COLUMN description,
    ADD COLUMN description_enc BYTEA NOT NULL;

-- ───────────────────────────────────────────────────────────────────────
-- 5. logical_transaction_state: encrypt current_sum, demote is_effective
-- ───────────────────────────────────────────────────────────────────────
-- is_effective can no longer be a GENERATED column because the expression
-- `current_sum <> 0` would need to operate on ciphertext. We demote it to
-- a plain Boolean column that the Rust service layer maintains explicitly
-- whenever it inserts a ledger row. The two indexes that reference this
-- column are recreated after the swap.

DROP INDEX IF EXISTS idx_lts_list_cursor;
DROP INDEX IF EXISTS idx_lts_user_effective;

ALTER TABLE logical_transaction_state
    DROP COLUMN is_effective,
    DROP COLUMN current_sum,
    ADD COLUMN current_sum_enc BYTEA NOT NULL,
    ADD COLUMN is_effective BOOLEAN NOT NULL;

CREATE INDEX idx_lts_user_effective ON logical_transaction_state (user_id, is_effective);
CREATE INDEX idx_lts_list_cursor
    ON logical_transaction_state (user_id, first_created_at DESC, id DESC)
    WHERE is_effective;

-- ───────────────────────────────────────────────────────────────────────
-- 6. account: encrypt balance + display metadata + allowance fields
-- ───────────────────────────────────────────────────────────────────────
-- The account.name unique constraint is dropped because it references the
-- plaintext column. Name uniqueness per user is no longer enforceable at
-- the DB level under encryption (ciphertext is randomized per-row with a
-- fresh nonce, so two rows with the same plaintext name produce different
-- ciphertext). The service layer enforces uniqueness by decrypting the
-- user's existing account names at write time.
ALTER TABLE account DROP CONSTRAINT IF EXISTS account_user_id_name_key;

ALTER TABLE account
    DROP COLUMN name,
    ADD COLUMN name_enc BYTEA NOT NULL,
    DROP COLUMN color,
    ADD COLUMN color_enc BYTEA NOT NULL,
    DROP COLUMN icon,
    ADD COLUMN icon_enc BYTEA NOT NULL,
    DROP COLUMN balance,
    ADD COLUMN current_balance_enc BYTEA NOT NULL,
    DROP COLUMN spend_limit,
    ADD COLUMN spend_limit_enc BYTEA,
    DROP COLUMN next_transfer_amount,
    ADD COLUMN next_transfer_amount_enc BYTEA,
    DROP COLUMN top_up_amount,
    ADD COLUMN top_up_amount_enc BYTEA;

-- ───────────────────────────────────────────────────────────────────────
-- 7. category: encrypt display metadata + description
-- ───────────────────────────────────────────────────────────────────────
-- Same name-uniqueness caveat as account.
ALTER TABLE category DROP CONSTRAINT IF EXISTS category_user_id_name_key;

ALTER TABLE category
    DROP COLUMN name,
    ADD COLUMN name_enc BYTEA NOT NULL,
    DROP COLUMN color,
    ADD COLUMN color_enc BYTEA,
    DROP COLUMN icon,
    ADD COLUMN icon_enc BYTEA,
    DROP COLUMN description,
    ADD COLUMN description_enc BYTEA;

-- ───────────────────────────────────────────────────────────────────────
-- 8. vendor: encrypt display fields
-- ───────────────────────────────────────────────────────────────────────
ALTER TABLE vendor DROP CONSTRAINT IF EXISTS vendor_user_id_name_key;

ALTER TABLE vendor
    DROP COLUMN name,
    ADD COLUMN name_enc BYTEA NOT NULL,
    DROP COLUMN description,
    ADD COLUMN description_enc BYTEA;

-- ───────────────────────────────────────────────────────────────────────
-- 9. budget_category: encrypt the budgeted value
-- ───────────────────────────────────────────────────────────────────────
ALTER TABLE budget_category
    DROP COLUMN budgeted_value,
    ADD COLUMN budgeted_value_enc BYTEA NOT NULL;

-- ───────────────────────────────────────────────────────────────────────
-- 10. subscription: encrypt name + billing_amount
-- ───────────────────────────────────────────────────────────────────────
-- The subscription auto-budget cron's responsibility for reading
-- billing_amount and upserting into budget_category is retired — the
-- client computes the monthly-normalized total from decrypted
-- subscriptions and displays it live.
ALTER TABLE subscription
    DROP COLUMN name,
    ADD COLUMN name_enc BYTEA NOT NULL,
    DROP COLUMN billing_amount,
    ADD COLUMN billing_amount_enc BYTEA NOT NULL;

-- ───────────────────────────────────────────────────────────────────────
-- 11. Retire overlays feature
-- ───────────────────────────────────────────────────────────────────────
-- Overlays are disabled for v1. Drop the tables rather than keeping
-- empty plaintext shells around; a future re-introduction will design
-- them from scratch under encryption.
DROP TABLE IF EXISTS overlay_transaction_inclusions;
DROP TABLE IF EXISTS overlay_category_caps;
DROP TABLE IF EXISTS overlays;
