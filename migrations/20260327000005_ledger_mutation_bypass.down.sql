-- Restore the unconditional immutability guard from the previous migration.
CREATE OR REPLACE FUNCTION transaction_immutability_guard()
    RETURNS TRIGGER
    LANGUAGE plpgsql AS
$$
BEGIN
    RAISE EXCEPTION 'ledger rows are immutable';
END;
$$;
