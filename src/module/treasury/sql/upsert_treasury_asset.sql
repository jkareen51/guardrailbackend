INSERT INTO treasury_asset_snapshots (
    asset_address,
    balance,
    reserved_yield,
    available_liquidity,
    updated_by_user_id,
    last_tx_hash
) VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (asset_address) DO UPDATE
SET
    balance = EXCLUDED.balance,
    reserved_yield = EXCLUDED.reserved_yield,
    available_liquidity = EXCLUDED.available_liquidity,
    updated_by_user_id = EXCLUDED.updated_by_user_id,
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, treasury_asset_snapshots.last_tx_hash),
    updated_at = NOW()
RETURNING
    asset_address,
    balance,
    reserved_yield,
    available_liquidity,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
