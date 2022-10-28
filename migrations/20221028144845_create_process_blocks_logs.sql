-- Add migration script here
CREATE TABLE processed_blocks_logs(
    id uuid PRIMARY KEY,
    chain_id SMALLINT UNIQUE NOT NULL,
    height BIGINT NOT NULL
)