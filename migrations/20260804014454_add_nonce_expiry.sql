-- Add migration script here
ALTER TABLE users ADD COLUMN nonce_issued_at TIMESTAMPTZ;