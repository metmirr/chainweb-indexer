-- Add migration script heredocker

CREATE TABLE blocks(
    id uuid PRIMARY KEY,
    chain_id SMALLINT NOT NULL,
    height BIGINT NOT NULL
);