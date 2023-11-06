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
    dissemination_num bigint NOT NULL,
    completion_num bigint NOT NULL,
    timeout_num bigint NOT NULL,
    score numeric(10, 2) NOT NULL,
    created_at timestamp without time zone NOT NULL,
    update_at timestamp without time zone NOT NULL
  );

ALTER TABLE assignment_user_summary ADD CONSTRAINT assignment_user_summary_pkey PRIMARY KEY (verification_address);



CREATE TABLE assignment_rewards (
    id serial NOT NULL,
    verification_address bytea NOT NULL,
    sender_address bytea NOT NULL,
    rewards numeric(80, 0) NULL,
    receive_status text NOT NULL,
    receive_time_at timestamp without time zone NULL,
    created_at timestamp without time zone NOT NULL DEFAULT now(),
    update_at timestamp without time zone NOT NULL DEFAULT now()
  );

ALTER TABLE assignment_rewards ADD CONSTRAINT assignment_rewards_pkey PRIMARY KEY (id);



CREATE TABLE assignment_penalties_detail (
    id serial NOT NULL,
    verification_address bytea NOT NULL,
    penalties_num numeric(80, 0) NOT NULL DEFAULT 0,
    type character varying(20) NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT now(),
    update_at timestamp without time zone NOT NULL DEFAULT now()
  );

ALTER TABLE assignment_penalties_detail ADD CONSTRAINT assignment_penalties_detail_pkey PRIMARY KEY (id);