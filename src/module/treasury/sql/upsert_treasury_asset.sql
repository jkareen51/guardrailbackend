INSERT INTO treasury_asset_snapshots (
    asset_address,
    balance,
    reserved_yield,
    reserved_redemptions,
    available_liquidity,
    registered_asset_token,
    updated_by_user_id,
    last_tx_hash
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (asset_address) DO UPDATE
SET
    balance = EXCLUDED.balance,
    reserved_yield = EXCLUDED.reserved_yield,
    reserved_redemptions = EXCLUDED.reserved_redemptions,
    available_liquidity = EXCLUDED.available_liquidity,
    registered_asset_token = EXCLUDED.registered_asset_token,
    updated_by_user_id = EXCLUDED.updated_by_user_id,
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, treasury_asset_snapshots.last_tx_hash),
    updated_at = NOW()
RETURNING
    asset_address,
    balance,
    reserved_yield,
    reserved_redemptions,
    available_liquidity,
    registered_asset_token,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
