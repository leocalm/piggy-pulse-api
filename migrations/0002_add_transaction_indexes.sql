-- Fix vendor_id to allow NULL (matches application model)
ALTER TABLE transaction ALTER COLUMN vendor_id DROP NOT NULL;

-- Add created_at column to transaction table (used in queries but missing from schema)
ALTER TABLE transaction ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Add indexes on transaction foreign keys for better JOIN performance
CREATE INDEX IF NOT EXISTS idx_transaction_category_id ON transaction(category_id);
CREATE INDEX IF NOT EXISTS idx_transaction_from_account_id ON transaction(from_account_id);
CREATE INDEX IF NOT EXISTS idx_transaction_to_account_id ON transaction(to_account_id);
CREATE INDEX IF NOT EXISTS idx_transaction_vendor_id ON transaction(vendor_id) WHERE vendor_id IS NOT NULL;

-- Add index on occurred_at for better ORDER BY performance
CREATE INDEX IF NOT EXISTS idx_transaction_occurred_at ON transaction(occurred_at DESC);

-- Add index on created_at for ORDER BY clauses
CREATE INDEX IF NOT EXISTS idx_transaction_created_at ON transaction(created_at DESC);

-- Composite index for period queries (occurred_at + created_at)
CREATE INDEX IF NOT EXISTS idx_transaction_occurred_created ON transaction(occurred_at DESC, created_at DESC);
