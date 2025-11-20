CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 1) Enum for AccountType
CREATE TYPE account_type AS ENUM (
    'Checking',
    'Savings',
    'CreditCard'
);

-- 2) Currency table
CREATE TABLE currency (
                          id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                          name TEXT NOT NULL,
                          symbol TEXT NOT NULL,
                          currency TEXT NOT NULL,
                          decimal_places SMALLINT NOT NULL,
                          created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                          updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 3) Account table
CREATE TABLE account (
                         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                         name TEXT NOT NULL,
                         color TEXT NOT NULL,
                         icon TEXT NOT NULL,
                         account_type account_type NOT NULL,
                         currency_id UUID NOT NULL REFERENCES currency(id) ON DELETE RESTRICT,
                         balance BIGINT NOT NULL,
                         created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                         updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 4) updated_at trigger
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_currency_updated_at
    BEFORE UPDATE ON currency
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER set_account_updated_at
    BEFORE UPDATE ON account
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();