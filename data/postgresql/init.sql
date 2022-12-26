CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE customer_keys (id UUID PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(), keplr_wallet_pubkey VARCHAR NOT NULL, project_id VARCHAR NOT NULL, token_ids TEXT[] NOT NULL);
CREATE UNIQUE INDEX keplr_wallet_project_idx ON customer_keys (keplr_wallet_pubkey, project_id);
