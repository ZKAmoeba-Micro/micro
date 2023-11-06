CREATE TABLE assignments (
    verification_address bytea NOT NULL,
    l1_batch_number bigint NOT NULL,
    status text NOT NULL,
    return_time_at timestamp(6) without time zone NULL,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);



CREATE UNIQUE INDEX assignments_un_address_l1batch ON assignments (verification_address, l1_batch_number);
