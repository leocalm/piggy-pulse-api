-- Change transaction.amount from INTEGER to BIGINT for consistency with account.balance
-- This allows large transaction amounts without overflow risk
ALTER TABLE transaction
    ALTER COLUMN amount TYPE BIGINT;

