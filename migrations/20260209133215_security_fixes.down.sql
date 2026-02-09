ALTER TABLE account
    DROP CONSTRAINT IF EXISTS account_user_id_name_key;
ALTER TABLE category
    DROP CONSTRAINT IF EXISTS category_user_id_name_key;
ALTER TABLE vendor
    DROP CONSTRAINT IF EXISTS vendor_user_id_name_key;
ALTER TABLE budget_period
    DROP CONSTRAINT IF EXISTS budget_period_user_id_name_key;

ALTER TABLE account
    ADD CONSTRAINT account_name_key UNIQUE (name);
ALTER TABLE category
    ADD CONSTRAINT category_name_key UNIQUE (name);
ALTER TABLE vendor
    ADD CONSTRAINT vendor_name_key UNIQUE (name);
ALTER TABLE budget_period
    ADD CONSTRAINT budget_period_name_key UNIQUE (name);

DROP TABLE IF EXISTS user_session;
