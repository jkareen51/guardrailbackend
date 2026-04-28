CREATE TABLE IF NOT EXISTS compliance_investors (
    wallet_address TEXT PRIMARY KEY,
    is_verified BOOLEAN NOT NULL,
    is_accredited BOOLEAN NOT NULL,
    is_frozen BOOLEAN NOT NULL,
    valid_until BIGINT NOT NULL DEFAULT 0,
    jurisdiction TEXT NOT NULL,
    external_ref TEXT NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS compliance_investors_updated_by_user_id_idx
ON compliance_investors (updated_by_user_id);

CREATE TABLE IF NOT EXISTS compliance_asset_rules (
    asset_address TEXT PRIMARY KEY,
    transfers_enabled BOOLEAN NOT NULL,
    subscriptions_enabled BOOLEAN NOT NULL,
    redemptions_enabled BOOLEAN NOT NULL,
    requires_accreditation BOOLEAN NOT NULL,
    min_investment TEXT NOT NULL,
    max_investor_balance TEXT NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS compliance_asset_rules_updated_by_user_id_idx
ON compliance_asset_rules (updated_by_user_id);

CREATE TABLE IF NOT EXISTS compliance_jurisdiction_restrictions (
    asset_address TEXT NOT NULL,
    jurisdiction TEXT NOT NULL,
    restricted BOOLEAN NOT NULL,
    updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    last_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (asset_address, jurisdiction)
);

CREATE INDEX IF NOT EXISTS compliance_jurisdiction_restrictions_updated_by_user_id_idx
ON compliance_jurisdiction_restrictions (updated_by_user_id);
