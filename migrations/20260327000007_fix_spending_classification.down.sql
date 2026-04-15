-- Revert to the Phase 1 trigger body (is_spending does not check
-- from_account_type). Does not touch user_daily_totals.spending — the data
-- remains under the corrected classification.
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

    IF NEW.to_account_id IS NOT NULL THEN
        SELECT a.account_type INTO to_acct_type FROM account a WHERE a.id = NEW.to_account_id;
    END IF;

    is_spending := (cat_type = 'Outgoing')
                OR (cat_type = 'Transfer' AND to_acct_type = 'Allowance');

    -- (The rest of the trigger body is identical to the Phase 1 version.)
    -- We intentionally do not duplicate it here; revert via the full up.sql
    -- if you need to rebuild from scratch.
    RAISE EXCEPTION 'Phase 3 down-migration is a stub; revert via migration 20260327000004 instead';
END;
$$;
