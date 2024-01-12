CREATE TABLE application_monitor (
    id SERIAL NOT NULL PRIMARY KEY ,
    app_name text NOT NULL,
    start_at timestamp(6) without time zone NOT NULL,
    ip text NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);
