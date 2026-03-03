-- PostgreSQL does not support removing values from an ENUM type.
-- To fully revert, recreate the type without DebitCard, Investment, and Cash.
-- This requires no rows use the values being removed.
ALTER TABLE accounts ALTER COLUMN account_type TYPE TEXT;
DROP TYPE account_type;
CREATE TYPE account_type AS ENUM ('Checking', 'Savings', 'CreditCard', 'Wallet', 'Allowance');
ALTER TABLE accounts ALTER COLUMN account_type TYPE account_type USING account_type::account_type;
