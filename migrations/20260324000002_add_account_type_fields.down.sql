ALTER TABLE account
    DROP COLUMN top_up_amount,
    DROP COLUMN top_up_cycle,
    DROP COLUMN top_up_day,
    DROP COLUMN statement_close_day,
    DROP COLUMN payment_due_day;
