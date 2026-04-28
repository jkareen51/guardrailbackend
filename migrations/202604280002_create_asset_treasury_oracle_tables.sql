CREATE TABLE IF NOT EXISTS asset_types (
    asset_type_id TEXT PRIMARY KEY,
    asset_type_name TEXT NOT NULL,
    implementation_address TEXT NOT NULL,
    is_registered BOOLEAN NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS asset_types_updated_by_user_id_idx
ON asset_types (updated_by_user_id);

CREATE TABLE IF NOT EXISTS assets (
    asset_address TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL UNIQUE,
    asset_type_id TEXT NOT NULL,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    max_supply TEXT NOT NULL,
    total_supply TEXT NOT NULL,
    asset_state INTEGER NOT NULL,
    asset_state_label TEXT NOT NULL,
    controllable BOOLEAN NOT NULL,
    self_service_purchase_enabled BOOLEAN NOT NULL,
    price_per_token TEXT NOT NULL,
    redemption_price_per_token TEXT NOT NULL,
    treasury_address TEXT NOT NULL,
    compliance_registry_address TEXT NOT NULL,
    payment_token_address TEXT NOT NULL,
    metadata_hash TEXT NOT NULL,
    holder_count TEXT NOT NULL,
    total_pending_redemptions TEXT NOT NULL,
    created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS assets_asset_type_id_idx
ON assets (asset_type_id);

CREATE INDEX IF NOT EXISTS assets_created_by_user_id_idx
ON assets (created_by_user_id);

CREATE INDEX IF NOT EXISTS assets_updated_by_user_id_idx
ON assets (updated_by_user_id);

CREATE TABLE IF NOT EXISTS treasury_status_snapshots (
    treasury_address TEXT PRIMARY KEY,
    payment_token_address TEXT NOT NULL,
    access_control_address TEXT NOT NULL,
    paused BOOLEAN NOT NULL,
    total_tracked_balance TEXT NOT NULL,
    total_reserved_yield TEXT NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS treasury_status_snapshots_updated_by_user_id_idx
ON treasury_status_snapshots (updated_by_user_id);

CREATE TABLE IF NOT EXISTS treasury_asset_snapshots (
    asset_address TEXT PRIMARY KEY,
    balance TEXT NOT NULL,
    reserved_yield TEXT NOT NULL,
    available_liquidity TEXT NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS treasury_asset_snapshots_updated_by_user_id_idx
ON treasury_asset_snapshots (updated_by_user_id);

CREATE TABLE IF NOT EXISTS oracle_trusted_oracles (
    oracle_address TEXT PRIMARY KEY,
    is_trusted BOOLEAN NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS oracle_trusted_oracles_updated_by_user_id_idx
ON oracle_trusted_oracles (updated_by_user_id);

CREATE TABLE IF NOT EXISTS oracle_valuations (
    asset_address TEXT PRIMARY KEY,
    asset_value TEXT NOT NULL,
    nav_per_token TEXT NOT NULL,
    onchain_updated_at BIGINT NOT NULL,
    reference_id TEXT NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS oracle_valuations_updated_by_user_id_idx
ON oracle_valuations (updated_by_user_id);

CREATE TABLE IF NOT EXISTS oracle_documents (
    asset_address TEXT NOT NULL,
    document_type TEXT NOT NULL,
    document_hash TEXT NOT NULL,
    reference_id TEXT NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (asset_address, document_type)
);

CREATE INDEX IF NOT EXISTS oracle_documents_updated_by_user_id_idx
ON oracle_documents (updated_by_user_id);
