-- Immutable Transaction Ledger
--
-- Converts the `transaction` table into an append-only ledger keyed by (id, seq),
-- introduces logical_transaction_state as the per-logical-transaction running state
-- (and the concurrency-control target for void/correct), and installs eight
-- trigger-maintained materialized aggregate tables plus an immutability trigger
-- that blocks UPDATE/DELETE on `transaction`.
--
-- See .kiro/specs/immutable-transaction-ledger/{requirements,design}.md for the
-- full spec. This migration is intended to run as a single atomic transaction.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- =============================================================================
-- 1. Composite PK (id, seq) and BIGINT amount on `transaction`
-- =============================================================================
--
-- Two tables currently have FKs pointing at transaction(id):
--   overlay_transaction_inclusions.transaction_id (ON DELETE CASCADE)
--   subscription_billing_event.transaction_id    (ON DELETE SET NULL)
--
-- Under the new composite PK (id, seq), a FK on `id` alone is no longer valid.
-- We drop those FKs here, swap the PK, and later (section 10) re-add them
-- pointing at logical_transaction_state(id), which remains a plain uniqueness
-- constraint. The ON DELETE clauses are preserved for semantic equivalence
-- even though state rows are never deleted in normal operation.

ALTER TABLE overlay_transaction_inclusions
    DROP CONSTRAINT overlay_transaction_inclusions_transaction_id_fkey;

ALTER TABLE subscription_billing_event
    DROP CONSTRAINT subscription_billing_event_transaction_id_fkey;

ALTER TABLE transaction ADD COLUMN seq BIGSERIAL NOT NULL;
ALTER TABLE transaction DROP CONSTRAINT transaction_pkey;
ALTER TABLE transaction ADD CONSTRAINT transaction_pkey PRIMARY KEY (id, seq);
ALTER TABLE transaction ALTER COLUMN amount TYPE BIGINT;

-- =============================================================================
-- 2. Immutability trigger on `transaction`
-- =============================================================================

CREATE OR REPLACE FUNCTION transaction_immutability_guard()
    RETURNS TRIGGER
    LANGUAGE plpgsql AS
$$
BEGIN
    RAISE EXCEPTION 'ledger rows are immutable';
END;
$$;

CREATE TRIGGER transaction_immutability
    BEFORE UPDATE OR DELETE
    ON transaction
    FOR EACH ROW
EXECUTE FUNCTION transaction_immutability_guard();

-- =============================================================================
-- 3. logical_transaction_state
-- =============================================================================

CREATE TABLE logical_transaction_state
(
    id               UUID        NOT NULL PRIMARY KEY,
    user_id          UUID        NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    current_sum      BIGINT      NOT NULL,
    is_effective     BOOLEAN     NOT NULL GENERATED ALWAYS AS (current_sum <> 0) STORED,
    latest_seq       BIGINT      NOT NULL,
    first_created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_lts_user_effective
    ON logical_transaction_state (user_id, is_effective);

CREATE INDEX idx_lts_list_cursor
    ON logical_transaction_state (user_id, first_created_at DESC, id DESC)
    WHERE is_effective;

-- =============================================================================
-- 4. Materialized aggregate tables (8 tables)
-- =============================================================================

CREATE TABLE account_balance_state
(
    account_id UUID    NOT NULL PRIMARY KEY REFERENCES account (id) ON DELETE CASCADE,
    sum_amount BIGINT  NOT NULL DEFAULT 0,
    tx_count   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE account_daily_delta
(
    account_id UUID    NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    day        DATE    NOT NULL,
    inflow     BIGINT  NOT NULL DEFAULT 0,
    outflow    BIGINT  NOT NULL DEFAULT 0,
    tx_count   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (account_id, day)
);

CREATE TABLE category_daily_spend
(
    category_id UUID    NOT NULL REFERENCES category (id) ON DELETE CASCADE,
    day         DATE    NOT NULL,
    sum_amount  BIGINT  NOT NULL DEFAULT 0,
    tx_count    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (category_id, day)
);

CREATE TABLE category_all_time
(
    category_id UUID    NOT NULL PRIMARY KEY REFERENCES category (id) ON DELETE CASCADE,
    sum_amount  BIGINT  NOT NULL DEFAULT 0,
    tx_count    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE vendor_daily_spend
(
    vendor_id  UUID    NOT NULL REFERENCES vendor (id) ON DELETE CASCADE,
    day        DATE    NOT NULL,
    sum_amount BIGINT  NOT NULL DEFAULT 0,
    tx_count   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (vendor_id, day)
);

CREATE TABLE vendor_all_time
(
    vendor_id  UUID    NOT NULL PRIMARY KEY REFERENCES vendor (id) ON DELETE CASCADE,
    sum_amount BIGINT  NOT NULL DEFAULT 0,
    tx_count   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE vendor_category_all_time
(
    vendor_id   UUID    NOT NULL REFERENCES vendor (id) ON DELETE CASCADE,
    category_id UUID    NOT NULL REFERENCES category (id) ON DELETE CASCADE,
    sum_amount  BIGINT  NOT NULL DEFAULT 0,
    tx_count    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (vendor_id, category_id)
);

CREATE TABLE user_daily_totals
(
    user_id             UUID    NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    day                 DATE    NOT NULL,
    inflow              BIGINT  NOT NULL DEFAULT 0,
    outflow             BIGINT  NOT NULL DEFAULT 0,
    spending            BIGINT  NOT NULL DEFAULT 0,
    tx_count            INTEGER NOT NULL DEFAULT 0,
    uncategorized_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, day)
);

-- =============================================================================
-- 5. Type immutability triggers on `category` and `account`
-- =============================================================================
--
-- category.category_type and account.account_type are snapshotted by the
-- aggregate maintenance trigger at insert time to classify inflow/outflow and
-- spending. Editing them after transactions exist would silently drift the
-- materialized aggregates, so we forbid edits at the DB layer.

CREATE OR REPLACE FUNCTION reject_category_type_change()
    RETURNS TRIGGER
    LANGUAGE plpgsql AS
$$
BEGIN
    IF NEW.category_type IS DISTINCT FROM OLD.category_type THEN
        RAISE EXCEPTION 'category.category_type is immutable after creation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER category_type_immutable
    BEFORE UPDATE
    ON category
    FOR EACH ROW
EXECUTE FUNCTION reject_category_type_change();

CREATE OR REPLACE FUNCTION reject_account_type_change()
    RETURNS TRIGGER
    LANGUAGE plpgsql AS
$$
BEGIN
    IF NEW.account_type IS DISTINCT FROM OLD.account_type THEN
        RAISE EXCEPTION 'account.account_type is immutable after creation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER account_type_immutable
    BEFORE UPDATE
    ON account
    FOR EACH ROW
EXECUTE FUNCTION reject_account_type_change();

-- =============================================================================
-- 6. Non-overlapping budget_period constraint
-- =============================================================================
--
-- Period-scoped reads sum day-keyed aggregates over [start_date, end_date].
-- Overlapping periods for the same user would double-count days, so we forbid
-- overlap at the DB layer via a GiST exclusion constraint.

ALTER TABLE budget_period
    ADD CONSTRAINT budget_period_no_overlap
        EXCLUDE USING gist (
            user_id WITH =,
            daterange(start_date, end_date, '[]') WITH &&
        );

-- =============================================================================
-- 7. Backfill logical_transaction_state and the 8 aggregate tables
-- =============================================================================
--
-- Pre-migration, every logical transaction `id` has exactly one row (the old PK
-- was `id` alone). So "effective" reduces to `amount <> 0`, and per-id metadata
-- is just the row's own columns. The backfill is one INSERT per destination
-- table, all reading from `transaction` directly.
--
-- The aggregate maintenance trigger is installed AFTER this block, so no
-- double-counting can occur.

-- 7.1 logical_transaction_state
INSERT INTO logical_transaction_state (id, user_id, current_sum, latest_seq, first_created_at)
SELECT id, user_id, amount::BIGINT, seq, created_at
FROM transaction;

-- 7.2 account_balance_state
-- The schema convention is:
--   Incoming: from_account_id = receiving account, to_account_id = NULL   → credit from
--   Outgoing: from_account_id = paying account,   to_account_id = NULL   → debit  from
--   Transfer: from_account_id = source,           to_account_id = dest   → debit from, credit to
-- Uncategorized transactions (category_id IS NULL) do not affect balances,
-- matching the legacy behavior in src/database/account.rs.
INSERT INTO account_balance_state (account_id, sum_amount, tx_count)
SELECT account_id,
       SUM(signed)::BIGINT   AS sum_amount,
       COUNT(DISTINCT tx_id) AS tx_count
FROM (
    -- Incoming: credit from_account_id
    SELECT t.from_account_id AS account_id, t.id AS tx_id, t.amount::BIGINT AS signed
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE t.amount <> 0 AND c.category_type = 'Incoming'
    UNION ALL
    -- Outgoing: debit from_account_id
    SELECT t.from_account_id, t.id, -t.amount::BIGINT
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE t.amount <> 0 AND c.category_type = 'Outgoing'
    UNION ALL
    -- Transfer: debit from_account_id
    SELECT t.from_account_id, t.id, -t.amount::BIGINT
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE t.amount <> 0 AND c.category_type = 'Transfer'
    UNION ALL
    -- Transfer: credit to_account_id
    SELECT t.to_account_id, t.id, t.amount::BIGINT
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE t.amount <> 0 AND c.category_type = 'Transfer' AND t.to_account_id IS NOT NULL
) flows
GROUP BY account_id;

-- 7.3 account_daily_delta
-- Same category-type-aware classification as 7.2, with flows split into
-- inflow/outflow columns per (account, day) bucket.
INSERT INTO account_daily_delta (account_id, day, inflow, outflow, tx_count)
SELECT account_id,
       day,
       SUM(inflow)::BIGINT   AS inflow,
       SUM(outflow)::BIGINT  AS outflow,
       COUNT(DISTINCT tx_id) AS tx_count
FROM (
    -- Incoming: inflow to from_account_id
    SELECT t.from_account_id AS account_id,
           t.occurred_at     AS day,
           t.id              AS tx_id,
           t.amount::BIGINT  AS inflow,
           0::BIGINT         AS outflow
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE t.amount <> 0 AND c.category_type = 'Incoming'
    UNION ALL
    -- Outgoing: outflow from from_account_id
    SELECT t.from_account_id,
           t.occurred_at,
           t.id,
           0::BIGINT,
           t.amount::BIGINT
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE t.amount <> 0 AND c.category_type = 'Outgoing'
    UNION ALL
    -- Transfer: outflow from from_account_id
    SELECT t.from_account_id,
           t.occurred_at,
           t.id,
           0::BIGINT,
           t.amount::BIGINT
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE t.amount <> 0 AND c.category_type = 'Transfer'
    UNION ALL
    -- Transfer: inflow to to_account_id
    SELECT t.to_account_id,
           t.occurred_at,
           t.id,
           t.amount::BIGINT,
           0::BIGINT
    FROM transaction t
    JOIN category c ON c.id = t.category_id
    WHERE t.amount <> 0 AND c.category_type = 'Transfer' AND t.to_account_id IS NOT NULL
) flows
GROUP BY account_id, day;

-- 7.4 category_daily_spend
INSERT INTO category_daily_spend (category_id, day, sum_amount, tx_count)
SELECT category_id,
       occurred_at AS day,
       SUM(amount)::BIGINT,
       COUNT(DISTINCT id)
FROM transaction
WHERE amount <> 0 AND category_id IS NOT NULL
GROUP BY category_id, occurred_at;

-- 7.5 category_all_time
INSERT INTO category_all_time (category_id, sum_amount, tx_count)
SELECT category_id,
       SUM(amount)::BIGINT,
       COUNT(DISTINCT id)
FROM transaction
WHERE amount <> 0 AND category_id IS NOT NULL
GROUP BY category_id;

-- 7.6 vendor_daily_spend
INSERT INTO vendor_daily_spend (vendor_id, day, sum_amount, tx_count)
SELECT vendor_id,
       occurred_at AS day,
       SUM(amount)::BIGINT,
       COUNT(DISTINCT id)
FROM transaction
WHERE amount <> 0 AND vendor_id IS NOT NULL
GROUP BY vendor_id, occurred_at;

-- 7.7 vendor_all_time
INSERT INTO vendor_all_time (vendor_id, sum_amount, tx_count)
SELECT vendor_id,
       SUM(amount)::BIGINT,
       COUNT(DISTINCT id)
FROM transaction
WHERE amount <> 0 AND vendor_id IS NOT NULL
GROUP BY vendor_id;

-- 7.8 vendor_category_all_time
INSERT INTO vendor_category_all_time (vendor_id, category_id, sum_amount, tx_count)
SELECT vendor_id,
       category_id,
       SUM(amount)::BIGINT,
       COUNT(DISTINCT id)
FROM transaction
WHERE amount <> 0 AND vendor_id IS NOT NULL AND category_id IS NOT NULL
GROUP BY vendor_id, category_id;

-- 7.9 user_daily_totals
INSERT INTO user_daily_totals (user_id, day, inflow, outflow, spending, tx_count, uncategorized_count)
SELECT t.user_id,
       t.occurred_at AS day,
       COALESCE(SUM(CASE WHEN c.category_type = 'Incoming' THEN t.amount ELSE 0 END), 0)::BIGINT,
       COALESCE(SUM(CASE WHEN c.category_type = 'Outgoing' THEN t.amount ELSE 0 END), 0)::BIGINT,
       COALESCE(SUM(CASE
                        WHEN c.category_type = 'Outgoing' THEN t.amount
                        WHEN c.category_type = 'Transfer' AND ta.account_type = 'Allowance' THEN t.amount
                        ELSE 0
                    END), 0)::BIGINT,
       COUNT(DISTINCT t.id),
       COUNT(DISTINCT t.id) FILTER (WHERE t.category_id IS NULL)
FROM transaction t
LEFT JOIN category c  ON c.id = t.category_id
LEFT JOIN account  ta ON ta.id = t.to_account_id
WHERE t.amount <> 0
GROUP BY t.user_id, t.occurred_at;

-- =============================================================================
-- 8. Aggregate maintenance trigger function
-- =============================================================================

CREATE OR REPLACE FUNCTION transaction_aggregate_maintenance()
    RETURNS TRIGGER
    LANGUAGE plpgsql AS
$$
DECLARE
    prev_sum        BIGINT;
    prev_effective  BOOLEAN;
    new_sum         BIGINT;
    new_effective   BOOLEAN;
    effective_delta INTEGER;
    cat_type        category_type;
    to_acct_type    account_type;
    is_spending     BOOLEAN;
    d               DATE := NEW.occurred_at;
BEGIN
    -- 1. Upsert logical_transaction_state, capturing the is_effective transition
    SELECT lts.current_sum, lts.is_effective
      INTO prev_sum, prev_effective
      FROM logical_transaction_state lts
     WHERE lts.id = NEW.id
       FOR UPDATE;

    IF NOT FOUND THEN
        INSERT INTO logical_transaction_state (id, user_id, current_sum, latest_seq, first_created_at)
        VALUES (NEW.id, NEW.user_id, NEW.amount, NEW.seq, NEW.created_at);
        new_sum := NEW.amount;
        new_effective := (new_sum <> 0);
        effective_delta := CASE WHEN new_effective THEN 1 ELSE 0 END;
    ELSE
        new_sum := prev_sum + NEW.amount;
        new_effective := (new_sum <> 0);
        effective_delta := CASE
            WHEN new_effective AND NOT prev_effective THEN 1
            WHEN NOT new_effective AND prev_effective THEN -1
            ELSE 0
        END;
        UPDATE logical_transaction_state
           SET current_sum = new_sum,
               latest_seq  = NEW.seq
         WHERE id = NEW.id;
    END IF;

    -- 2. Resolve classification (snapshotted at insert time)
    IF NEW.category_id IS NOT NULL THEN
        SELECT c.category_type INTO cat_type FROM category c WHERE c.id = NEW.category_id;
    END IF;

    IF NEW.to_account_id IS NOT NULL THEN
        SELECT a.account_type INTO to_acct_type FROM account a WHERE a.id = NEW.to_account_id;
    END IF;

    is_spending := (cat_type = 'Outgoing')
                OR (cat_type = 'Transfer' AND to_acct_type = 'Allowance');

    -- 3. account_balance_state + account_daily_delta
    --
    -- Schema convention (see section 7.2 comment):
    --   Incoming: credit  from_account_id
    --   Outgoing: debit   from_account_id
    --   Transfer: debit   from_account_id AND credit to_account_id
    --   Uncategorized (cat_type IS NULL): no balance effect
    IF cat_type = 'Incoming' THEN
        INSERT INTO account_balance_state (account_id, sum_amount, tx_count)
        VALUES (NEW.from_account_id, NEW.amount, effective_delta)
        ON CONFLICT (account_id) DO UPDATE
           SET sum_amount = account_balance_state.sum_amount + EXCLUDED.sum_amount,
               tx_count   = account_balance_state.tx_count   + EXCLUDED.tx_count;

        INSERT INTO account_daily_delta (account_id, day, inflow, outflow, tx_count)
        VALUES (NEW.from_account_id, d, NEW.amount, 0, effective_delta)
        ON CONFLICT (account_id, day) DO UPDATE
           SET inflow   = account_daily_delta.inflow   + EXCLUDED.inflow,
               tx_count = account_daily_delta.tx_count + EXCLUDED.tx_count;

    ELSIF cat_type = 'Outgoing' THEN
        INSERT INTO account_balance_state (account_id, sum_amount, tx_count)
        VALUES (NEW.from_account_id, -NEW.amount, effective_delta)
        ON CONFLICT (account_id) DO UPDATE
           SET sum_amount = account_balance_state.sum_amount + EXCLUDED.sum_amount,
               tx_count   = account_balance_state.tx_count   + EXCLUDED.tx_count;

        INSERT INTO account_daily_delta (account_id, day, inflow, outflow, tx_count)
        VALUES (NEW.from_account_id, d, 0, NEW.amount, effective_delta)
        ON CONFLICT (account_id, day) DO UPDATE
           SET outflow  = account_daily_delta.outflow  + EXCLUDED.outflow,
               tx_count = account_daily_delta.tx_count + EXCLUDED.tx_count;

    ELSIF cat_type = 'Transfer' THEN
        -- Debit from_account
        INSERT INTO account_balance_state (account_id, sum_amount, tx_count)
        VALUES (NEW.from_account_id, -NEW.amount, effective_delta)
        ON CONFLICT (account_id) DO UPDATE
           SET sum_amount = account_balance_state.sum_amount + EXCLUDED.sum_amount,
               tx_count   = account_balance_state.tx_count   + EXCLUDED.tx_count;

        INSERT INTO account_daily_delta (account_id, day, inflow, outflow, tx_count)
        VALUES (NEW.from_account_id, d, 0, NEW.amount, effective_delta)
        ON CONFLICT (account_id, day) DO UPDATE
           SET outflow  = account_daily_delta.outflow  + EXCLUDED.outflow,
               tx_count = account_daily_delta.tx_count + EXCLUDED.tx_count;

        -- Credit to_account
        IF NEW.to_account_id IS NOT NULL THEN
            INSERT INTO account_balance_state (account_id, sum_amount, tx_count)
            VALUES (NEW.to_account_id, NEW.amount, effective_delta)
            ON CONFLICT (account_id) DO UPDATE
               SET sum_amount = account_balance_state.sum_amount + EXCLUDED.sum_amount,
                   tx_count   = account_balance_state.tx_count   + EXCLUDED.tx_count;

            INSERT INTO account_daily_delta (account_id, day, inflow, outflow, tx_count)
            VALUES (NEW.to_account_id, d, NEW.amount, 0, effective_delta)
            ON CONFLICT (account_id, day) DO UPDATE
               SET inflow   = account_daily_delta.inflow   + EXCLUDED.inflow,
                   tx_count = account_daily_delta.tx_count + EXCLUDED.tx_count;
        END IF;
    END IF;

    -- 5. category_daily_spend + category_all_time
    IF NEW.category_id IS NOT NULL THEN
        INSERT INTO category_daily_spend (category_id, day, sum_amount, tx_count)
        VALUES (NEW.category_id, d, NEW.amount, effective_delta)
        ON CONFLICT (category_id, day) DO UPDATE
           SET sum_amount = category_daily_spend.sum_amount + EXCLUDED.sum_amount,
               tx_count   = category_daily_spend.tx_count   + EXCLUDED.tx_count;

        INSERT INTO category_all_time (category_id, sum_amount, tx_count)
        VALUES (NEW.category_id, NEW.amount, effective_delta)
        ON CONFLICT (category_id) DO UPDATE
           SET sum_amount = category_all_time.sum_amount + EXCLUDED.sum_amount,
               tx_count   = category_all_time.tx_count   + EXCLUDED.tx_count;
    END IF;

    -- 6. vendor_daily_spend + vendor_all_time + vendor_category_all_time
    IF NEW.vendor_id IS NOT NULL THEN
        INSERT INTO vendor_daily_spend (vendor_id, day, sum_amount, tx_count)
        VALUES (NEW.vendor_id, d, NEW.amount, effective_delta)
        ON CONFLICT (vendor_id, day) DO UPDATE
           SET sum_amount = vendor_daily_spend.sum_amount + EXCLUDED.sum_amount,
               tx_count   = vendor_daily_spend.tx_count   + EXCLUDED.tx_count;

        INSERT INTO vendor_all_time (vendor_id, sum_amount, tx_count)
        VALUES (NEW.vendor_id, NEW.amount, effective_delta)
        ON CONFLICT (vendor_id) DO UPDATE
           SET sum_amount = vendor_all_time.sum_amount + EXCLUDED.sum_amount,
               tx_count   = vendor_all_time.tx_count   + EXCLUDED.tx_count;

        IF NEW.category_id IS NOT NULL THEN
            INSERT INTO vendor_category_all_time (vendor_id, category_id, sum_amount, tx_count)
            VALUES (NEW.vendor_id, NEW.category_id, NEW.amount, effective_delta)
            ON CONFLICT (vendor_id, category_id) DO UPDATE
               SET sum_amount = vendor_category_all_time.sum_amount + EXCLUDED.sum_amount,
                   tx_count   = vendor_category_all_time.tx_count   + EXCLUDED.tx_count;
        END IF;
    END IF;

    -- 7. user_daily_totals
    INSERT INTO user_daily_totals (user_id, day, inflow, outflow, spending, tx_count, uncategorized_count)
    VALUES (
        NEW.user_id,
        d,
        CASE WHEN cat_type = 'Incoming' THEN NEW.amount ELSE 0 END,
        CASE WHEN cat_type = 'Outgoing' THEN NEW.amount ELSE 0 END,
        CASE WHEN is_spending THEN NEW.amount ELSE 0 END,
        effective_delta,
        CASE WHEN NEW.category_id IS NULL THEN effective_delta ELSE 0 END
    )
    ON CONFLICT (user_id, day) DO UPDATE
       SET inflow              = user_daily_totals.inflow              + EXCLUDED.inflow,
           outflow             = user_daily_totals.outflow             + EXCLUDED.outflow,
           spending            = user_daily_totals.spending            + EXCLUDED.spending,
           tx_count            = user_daily_totals.tx_count            + EXCLUDED.tx_count,
           uncategorized_count = user_daily_totals.uncategorized_count + EXCLUDED.uncategorized_count;

    RETURN NEW;
END;
$$;

CREATE TRIGGER transaction_aggregate_maintain
    AFTER INSERT
    ON transaction
    FOR EACH ROW
EXECUTE FUNCTION transaction_aggregate_maintenance();

-- =============================================================================
-- 9. Re-add FKs to logical_transaction_state(id)
-- =============================================================================
--
-- These replace the FKs dropped in section 1. Every transaction.id now has
-- a corresponding logical_transaction_state.id row (backfilled in 7.1), so
-- this is safe for existing data.

ALTER TABLE overlay_transaction_inclusions
    ADD CONSTRAINT overlay_transaction_inclusions_transaction_id_fkey
        FOREIGN KEY (transaction_id) REFERENCES logical_transaction_state (id) ON DELETE CASCADE;

ALTER TABLE subscription_billing_event
    ADD CONSTRAINT subscription_billing_event_transaction_id_fkey
        FOREIGN KEY (transaction_id) REFERENCES logical_transaction_state (id) ON DELETE SET NULL;

-- =============================================================================
-- 10. Backfill verification
-- =============================================================================
--
-- Each check recomputes an aggregate from the ledger and compares to the
-- materialized value. Any discrepancy raises an exception and aborts the
-- migration.

DO
$$
DECLARE
    mismatch_count INTEGER;
BEGIN
    -- logical_transaction_state
    SELECT COUNT(*) INTO mismatch_count
    FROM transaction t
    LEFT JOIN logical_transaction_state lts ON lts.id = t.id
    WHERE lts.current_sum IS DISTINCT FROM t.amount::BIGINT;
    IF mismatch_count > 0 THEN
        RAISE EXCEPTION 'backfill verification failed: logical_transaction_state (% mismatches)', mismatch_count;
    END IF;

    -- category_all_time
    SELECT COUNT(*) INTO mismatch_count
    FROM (
        SELECT category_id,
               SUM(amount)::BIGINT AS expected_sum,
               COUNT(DISTINCT id)  AS expected_count
        FROM transaction
        WHERE amount <> 0 AND category_id IS NOT NULL
        GROUP BY category_id
    ) expected
    LEFT JOIN category_all_time actual USING (category_id)
    WHERE COALESCE(actual.sum_amount, 0) IS DISTINCT FROM expected.expected_sum
       OR COALESCE(actual.tx_count, 0)   IS DISTINCT FROM expected.expected_count;
    IF mismatch_count > 0 THEN
        RAISE EXCEPTION 'backfill verification failed: category_all_time (% mismatches)', mismatch_count;
    END IF;

    -- vendor_all_time
    SELECT COUNT(*) INTO mismatch_count
    FROM (
        SELECT vendor_id,
               SUM(amount)::BIGINT AS expected_sum,
               COUNT(DISTINCT id)  AS expected_count
        FROM transaction
        WHERE amount <> 0 AND vendor_id IS NOT NULL
        GROUP BY vendor_id
    ) expected
    LEFT JOIN vendor_all_time actual USING (vendor_id)
    WHERE COALESCE(actual.sum_amount, 0) IS DISTINCT FROM expected.expected_sum
       OR COALESCE(actual.tx_count, 0)   IS DISTINCT FROM expected.expected_count;
    IF mismatch_count > 0 THEN
        RAISE EXCEPTION 'backfill verification failed: vendor_all_time (% mismatches)', mismatch_count;
    END IF;

    -- account_balance_state parity vs legacy category-type-weighted balance query
    SELECT COUNT(*) INTO mismatch_count
    FROM (
        SELECT a.id AS account_id,
               COALESCE(SUM(
                   CASE
                       WHEN c.category_type = 'Incoming'                              THEN  t.amount::BIGINT
                       WHEN c.category_type = 'Outgoing'                              THEN -t.amount::BIGINT
                       WHEN c.category_type = 'Transfer' AND t.from_account_id = a.id THEN -t.amount::BIGINT
                       WHEN c.category_type = 'Transfer' AND t.to_account_id   = a.id THEN  t.amount::BIGINT
                       ELSE 0
                   END
               ), 0) AS legacy_sum
        FROM account a
        LEFT JOIN transaction t ON (t.from_account_id = a.id OR t.to_account_id = a.id)
        LEFT JOIN category c ON c.id = t.category_id
        GROUP BY a.id
    ) expected
    LEFT JOIN account_balance_state actual ON actual.account_id = expected.account_id
    WHERE COALESCE(actual.sum_amount, 0) IS DISTINCT FROM expected.legacy_sum;
    IF mismatch_count > 0 THEN
        RAISE EXCEPTION 'backfill verification failed: account_balance_state parity (% mismatches)', mismatch_count;
    END IF;

    -- user_daily_totals.spending (most complex classification — worth verifying)
    SELECT COUNT(*) INTO mismatch_count
    FROM (
        SELECT t.user_id,
               t.occurred_at AS day,
               COALESCE(SUM(CASE
                                WHEN c.category_type = 'Outgoing' THEN t.amount
                                WHEN c.category_type = 'Transfer' AND ta.account_type = 'Allowance' THEN t.amount
                                ELSE 0
                            END), 0)::BIGINT AS expected_spending
        FROM transaction t
        LEFT JOIN category c  ON c.id  = t.category_id
        LEFT JOIN account  ta ON ta.id = t.to_account_id
        WHERE t.amount <> 0
        GROUP BY t.user_id, t.occurred_at
    ) expected
    LEFT JOIN user_daily_totals actual
           ON actual.user_id = expected.user_id AND actual.day = expected.day
    WHERE COALESCE(actual.spending, 0) IS DISTINCT FROM expected.expected_spending;
    IF mismatch_count > 0 THEN
        RAISE EXCEPTION 'backfill verification failed: user_daily_totals.spending (% mismatches)', mismatch_count;
    END IF;
END
$$;
