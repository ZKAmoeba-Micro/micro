CREATE TABLE assignments (
    verification_address bytea NOT NULL,
    l1_batch_number bigint NOT NULL,
    status text NOT NULL,
    return_time_at timestamp(6) without time zone NULL,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);



CREATE UNIQUE INDEX assignments_un_address_l1batch ON assignments (verification_address, l1_batch_number);
CREATE INDEX assignments_idx_status ON assignments (status);



CREATE TABLE assignment_user_summary (
    verification_address bytea NOT NULL,
    status text NOT NULL,
    participations_num bigint NOT NULL,
    completion_num bigint NOT NULL,
    timeout_num bigint NOT NULL,
    score numeric(10, 2) NOT NULL,
    created_at timestamp without time zone NOT NULL,
    update_at timestamp without time zone NOT NULL,
    deposit_amount numeric(38, 18) NOT NULL,
    total_prove_time bigint NOT NULL DEFAULT 0
  );

ALTER TABLE assignment_user_summary ADD CONSTRAINT assignment_user_summary_pkey PRIMARY KEY (verification_address);

