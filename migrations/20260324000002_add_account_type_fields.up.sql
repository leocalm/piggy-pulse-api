ALTER TABLE account
    ADD COLUMN top_up_amount BIGINT NULL,
    ADD COLUMN top_up_cycle VARCHAR(20) NULL,
    ADD COLUMN top_up_day SMALLINT NULL,
    ADD COLUMN statement_close_day SMALLINT NULL,
    ADD COLUMN payment_due_day SMALLINT NULL;
