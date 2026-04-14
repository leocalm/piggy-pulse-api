-- Fix two classification bugs that came out of the Phase 3 dashboard audit.
--
-- (1) `user_daily_totals.spending` should exclude Outgoing-from-Allowance.
--     The Phase 1 trigger defined `is_spending` as:
--         (cat_type = 'Outgoing') OR (cat_type = 'Transfer' AND to = 'Allowance')
--     But every legacy dashboard query (get_current_period_dashboard,
--     get_daily_spend_v2, get_cash_flow_v2, get_spending_trend_v2,
--     get_budget_stability_v2) uses the stricter definition:
--         (cat_type = 'Outgoing' AND from_account_type <> 'Allowance')
--      OR (cat_type = 'Transfer'  AND to_account_type   = 'Allowance')
--     The intent is that money leaving an Allowance account is already
--     accounted for by the Allowance top-up, so re-counting it as spending
--     double-counts. The Phase 1 trigger was wrong.
--
-- (2) `vendor_daily_spend` should only accept Outgoing-from-non-Allowance
--     transactions. Vendors in PiggyPulse model purchase counterparties, so
--     only outgoing money to them counts as "vendor spend". The Phase 1
--     trigger accepted any vendor_id-bearing row regardless of category, which
--     would let a refund (Incoming with vendor) or an internal transfer
--     inflate the top-vendors card. `get_top_vendors_v2` in dashboard_v2.rs
--     compensates with a category_type filter today, but moving the filter
--     into the trigger lets the read path drop the `category` join entirely.
--     `vendor_all_time` and `vendor_category_all_time` use the same filter
--     for consistency.
--
-- This migration:
--   1. Updates the trigger function body to reflect both fixes
--   2. Recomputes user_daily_totals.spending, vendor_daily_spend,
--      vendor_all_time, and vendor_category_all_time from the ledger

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
    from_acct_type  account_type;
    to_acct_type    account_type;
    is_spending     BOOLEAN;
    d               DATE := NEW.occurred_at;
BEGIN
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

    IF NEW.category_id IS NOT NULL THEN
        SELECT c.category_type INTO cat_type FROM category c WHERE c.id = NEW.category_id;
    END IF;

    SELECT a.account_type INTO from_acct_type FROM account a WHERE a.id = NEW.from_account_id;

    IF NEW.to_account_id IS NOT NULL THEN
        SELECT a.account_type INTO to_acct_type FROM account a WHERE a.id = NEW.to_account_id;
    END IF;

    is_spending := (cat_type = 'Outgoing' AND from_acct_type <> 'Allowance')
                OR (cat_type = 'Transfer' AND to_acct_type = 'Allowance');

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

    -- Vendor aggregates only record Outgoing-from-non-Allowance flows, which
    -- is the product definition of "vendor spend". Other combinations
    -- (Incoming refunds, Transfers, purchases from an Allowance) are not
    -- counted, matching the legacy dashboard query filters.
    IF NEW.vendor_id IS NOT NULL AND cat_type = 'Outgoing' AND from_acct_type <> 'Allowance' THEN
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

-- Recompute user_daily_totals.spending from the ledger. Only the `spending`
-- column changes — inflow, outflow, tx_count, and uncategorized_count are
-- unaffected by the classification fix.
UPDATE user_daily_totals udt
   SET spending = COALESCE(sub.spending, 0)
  FROM (
    SELECT t.user_id,
           t.occurred_at AS day,
           COALESCE(SUM(CASE
                            WHEN c.category_type = 'Outgoing' AND fa.account_type <> 'Allowance' THEN t.amount
                            WHEN c.category_type = 'Transfer' AND ta.account_type = 'Allowance'  THEN t.amount
                            ELSE 0
                        END), 0)::BIGINT AS spending
      FROM transaction t
      JOIN logical_transaction_state lts ON lts.id = t.id AND t.seq = lts.latest_seq AND lts.is_effective
      LEFT JOIN category c  ON c.id  = t.category_id
      LEFT JOIN account  fa ON fa.id = t.from_account_id
      LEFT JOIN account  ta ON ta.id = t.to_account_id
     GROUP BY t.user_id, t.occurred_at
  ) sub
 WHERE udt.user_id = sub.user_id AND udt.day = sub.day;

UPDATE user_daily_totals udt
   SET spending = 0
 WHERE udt.spending <> 0
   AND NOT EXISTS (
       SELECT 1
         FROM transaction t
         JOIN logical_transaction_state lts ON lts.id = t.id AND t.seq = lts.latest_seq AND lts.is_effective
         LEFT JOIN category c  ON c.id  = t.category_id
         LEFT JOIN account  fa ON fa.id = t.from_account_id
         LEFT JOIN account  ta ON ta.id = t.to_account_id
        WHERE t.user_id = udt.user_id
          AND t.occurred_at = udt.day
          AND ((c.category_type = 'Outgoing' AND fa.account_type <> 'Allowance')
               OR (c.category_type = 'Transfer' AND ta.account_type = 'Allowance'))
   );

-- Rebuild vendor aggregates from scratch with the corrected classification.
-- Only Outgoing-from-non-Allowance rows count. Backfill is idempotent: we
-- truncate and re-insert from the ledger's Latest_Row view.
TRUNCATE TABLE vendor_daily_spend, vendor_all_time, vendor_category_all_time;

INSERT INTO vendor_daily_spend (vendor_id, day, sum_amount, tx_count)
SELECT t.vendor_id,
       t.occurred_at,
       SUM(t.amount)::BIGINT,
       COUNT(DISTINCT t.id)
  FROM transaction t
  JOIN logical_transaction_state lts ON lts.id = t.id AND t.seq = lts.latest_seq AND lts.is_effective
  JOIN category c ON c.id = t.category_id
  JOIN account  fa ON fa.id = t.from_account_id
 WHERE t.vendor_id IS NOT NULL
   AND c.category_type = 'Outgoing'
   AND fa.account_type <> 'Allowance'
 GROUP BY t.vendor_id, t.occurred_at;

INSERT INTO vendor_all_time (vendor_id, sum_amount, tx_count)
SELECT t.vendor_id,
       SUM(t.amount)::BIGINT,
       COUNT(DISTINCT t.id)
  FROM transaction t
  JOIN logical_transaction_state lts ON lts.id = t.id AND t.seq = lts.latest_seq AND lts.is_effective
  JOIN category c ON c.id = t.category_id
  JOIN account  fa ON fa.id = t.from_account_id
 WHERE t.vendor_id IS NOT NULL
   AND c.category_type = 'Outgoing'
   AND fa.account_type <> 'Allowance'
 GROUP BY t.vendor_id;

INSERT INTO vendor_category_all_time (vendor_id, category_id, sum_amount, tx_count)
SELECT t.vendor_id,
       t.category_id,
       SUM(t.amount)::BIGINT,
       COUNT(DISTINCT t.id)
  FROM transaction t
  JOIN logical_transaction_state lts ON lts.id = t.id AND t.seq = lts.latest_seq AND lts.is_effective
  JOIN category c ON c.id = t.category_id
  JOIN account  fa ON fa.id = t.from_account_id
 WHERE t.vendor_id IS NOT NULL
   AND c.category_type = 'Outgoing'
   AND fa.account_type <> 'Allowance'
 GROUP BY t.vendor_id, t.category_id;
