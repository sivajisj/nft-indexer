-- Add migration script here
CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL,
    contract_address VARCHAR(42) NOT NULL,
    event_type TEXT NOT NULL,
    token_id NUMERIC NOT NULL,
    from_address VARCHAR(42),
    to_address VARCHAR(42),
    block_number BIGINT NOT NULL,
    tx_hash VARCHAR(66) NOT NULL,
    log_index INTEGER NOT NULL,
    confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (tx_hash, log_index)

);
CREATE INDEX idx_events_confirmed ON events(confirmed);