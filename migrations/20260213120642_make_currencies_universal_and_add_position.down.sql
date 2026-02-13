-- 1. Drop the restrict FK
ALTER TABLE account DROP CONSTRAINT account_currency_id_fkey;

-- 2. Truncate tables again to start fresh (cannot restore lost data)
TRUNCATE TABLE account CASCADE;
TRUNCATE TABLE currency CASCADE;

-- 3. Restore schema
ALTER TABLE currency
    DROP COLUMN symbol_position,
    ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE CASCADE;

-- 4. Restore original FK
ALTER TABLE account
    ADD CONSTRAINT account_currency_id_fkey
    FOREIGN KEY (currency_id)
    REFERENCES currency(id)
    ON DELETE CASCADE;
