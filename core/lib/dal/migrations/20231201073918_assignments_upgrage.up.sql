DROP INDEX IF EXISTS assignments_idx_status;
DROP INDEX IF EXISTS assignments_unkey;

DROP TABLE IF EXISTS assignments;
DROP TABLE IF EXISTS assignment_user_summary;



CREATE TABLE assignments (
    id SERIAL NOT NULL PRIMARY KEY ,
    verification_address bytea NOT NULL,
    l1_batch_number bigint NOT NULL,
    miniblock_number bigint default 0 NOT NULL,
    storage_index  bigint NOT NULL,
    tx_hash bytea  NULL,
    status text NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

CREATE INDEX assignments_idx_status ON assignments (status);
CREATE UNIQUE INDEX assignments_unkey ON assignments (verification_address,l1_batch_number,storage_index);

