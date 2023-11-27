CREATE TABLE assignments (
    id SERIAL NOT NULL PRIMARY KEY ,
    verification_address bytea NOT NULL,
    l1_batch_number bigint NOT NULL,
    tx_hash bytea  NULL,
    status text NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

CREATE INDEX assignments_idx_status ON assignments (status);



CREATE TABLE assignment_user_summary (
    verification_address bytea NOT NULL,
    status text NOT NULL,
    base_score integer NOT NULL,
    last_time bigint NOT NULL,
    miniblock_number bigint default 0 NOT NULL,
    created_at timestamp without time zone NOT NULL,
    update_at timestamp without time zone NOT NULL
  );

ALTER TABLE assignment_user_summary ADD CONSTRAINT assignment_user_summary_pkey PRIMARY KEY (verification_address);

