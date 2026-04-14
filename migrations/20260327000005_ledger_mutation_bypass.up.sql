-- Ledger immutability bypass via a session-scoped GUC.
--
-- The `transaction_immutability` trigger blocks every UPDATE/DELETE on the
-- `transaction` table, which also blocks legitimate cascade deletes from
-- parent tables (account, category, vendor) during bulk reset operations
-- like reset_structure_v2 and delete_all_user_data.
--
-- This migration replaces the trigger function so that it honors a custom
-- GUC `piggy_pulse.allow_ledger_mutations`. When set to 'on' for the current
-- transaction (via `SET LOCAL piggy_pulse.allow_ledger_mutations = 'on'`),
-- UPDATE/DELETE statements pass through. Unset or 'off' restores the ledger
-- guarantee.
--
-- Semantics:
--   - The GUC is a custom namespace ("piggy_pulse.*"), so it is isolated
--     from Postgres built-ins and cannot collide with other settings.
--   - `SET LOCAL` scopes the bypass to the current database transaction; the
--     commit or rollback that follows clears it automatically. This prevents
--     connection-pool leakage of the bypass state.
--   - Callers must explicitly opt in; a forgotten `SET LOCAL` means the
--     trigger re-raises "ledger rows are immutable" as before.

CREATE OR REPLACE FUNCTION transaction_immutability_guard()
    RETURNS TRIGGER
    LANGUAGE plpgsql AS
$$
BEGIN
    IF current_setting('piggy_pulse.allow_ledger_mutations', true) = 'on' THEN
        RETURN COALESCE(OLD, NEW);
    END IF;
    RAISE EXCEPTION 'ledger rows are immutable';
END;
$$;
