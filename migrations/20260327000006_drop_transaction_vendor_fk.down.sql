-- Restore the transaction.vendor_id foreign key.
--
-- Note: if any merge_vendor runs have left dangling vendor_id values in
-- transaction (rows whose vendor_id no longer exists in the vendor table),
-- this will fail with a 23503 foreign_key_violation. In that case the
-- operator must manually null out or repair those rows before reverting.
ALTER TABLE transaction
    ADD CONSTRAINT transaction_vendor_id_fkey
        FOREIGN KEY (vendor_id) REFERENCES vendor (id) ON DELETE CASCADE;
