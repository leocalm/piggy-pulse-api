-- 1. Drop the existing foreign key constraint on account
ALTER TABLE account DROP CONSTRAINT account_currency_id_fkey;

-- 2. Clear the currency table.
-- IMPORTANT: Since we dropped the FK constraint above, we need to handle the data integrity.
-- However, since the user chose "Option C" (Dev/Reset), we want to wipe the slate clean.
-- But wait, if I dropped the FK, deleting currency won't auto-delete accounts anymore.
-- So I should delete accounts first to respect the "clean slate" and avoid orphan records.
TRUNCATE TABLE account CASCADE;
TRUNCATE TABLE currency CASCADE;

-- 3. Modify currency table structure
ALTER TABLE currency
    ADD COLUMN symbol_position TEXT NOT NULL CHECK (symbol_position IN ('before', 'after')) DEFAULT 'before',
    DROP COLUMN user_id;

-- 4. Re-add the foreign key constraint on account with ON DELETE RESTRICT
ALTER TABLE account
    ADD CONSTRAINT account_currency_id_fkey
    FOREIGN KEY (currency_id)
    REFERENCES currency(id)
    ON DELETE RESTRICT;

-- 5. Insert default currencies
INSERT INTO currency (name, symbol, currency, decimal_places, symbol_position)
VALUES
    ('Brazilian Real', 'R$', 'BRL', 2, 'before'),
    ('Euro', '€', 'EUR', 2, 'before'),
    ('Yen', '¥', 'JPY', 0, 'after');
