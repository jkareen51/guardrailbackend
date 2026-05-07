-- Create user trade history table to track all asset purchases and redemptions
CREATE TABLE IF NOT EXISTS user_trade_history (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    wallet_address TEXT NOT NULL,
    asset_address TEXT NOT NULL REFERENCES assets(asset_address) ON DELETE CASCADE,
    trade_type TEXT NOT NULL CHECK (trade_type IN ('purchase', 'redemption', 'redemption_processed', 'redemption_cancelled')),
    token_amount TEXT NOT NULL,
    payment_amount TEXT NOT NULL,
    price_per_token TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying user's trade history
CREATE INDEX IF NOT EXISTS user_trade_history_user_id_executed_at_idx
ON user_trade_history (user_id, executed_at DESC);

-- Index for querying by wallet
CREATE INDEX IF NOT EXISTS user_trade_history_wallet_address_executed_at_idx
ON user_trade_history (wallet_address, executed_at DESC);

-- Index for querying by asset
CREATE INDEX IF NOT EXISTS user_trade_history_asset_address_executed_at_idx
ON user_trade_history (asset_address, executed_at DESC);

-- Unique constraint on tx_hash to prevent duplicates
CREATE UNIQUE INDEX IF NOT EXISTS user_trade_history_tx_hash_uidx
ON user_trade_history (tx_hash);
