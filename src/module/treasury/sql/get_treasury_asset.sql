SELECT
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
FROM treasury_asset_snapshots
WHERE asset_address = $1
