DROP INDEX IF EXISTS assignments_unkey;
ALTER TABLE assignments ADD batch_hash bytea NULL;
CREATE UNIQUE INDEX assignments_unkey ON assignments (verification_address,l1_batch_number,storage_index,batch_hash);