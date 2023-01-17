CREATE TYPE migration_status_values AS ENUM('pending', 'processing', 'success', 'error');

CREATE TABLE migration_queue (id UUID PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(), keplr_wallet_pubkey VARCHAR NOT NULL, project_id VARCHAR NOT NULL, token_id VARCHAR(10) NOT NULL, transaction_hash VARCHAR DEFAULT NULL, migration_status migration_status_values NOT NULL DEFAULT 'pending');
CREATE UNIQUE INDEX migration_item_idx ON migration_queue (keplr_wallet_pubkey, project_id, token_id);
ALTER TABLE migration_queue ADD starknet_wallet_pubkey VARCHAR NOT NULL DEFAULT '';
