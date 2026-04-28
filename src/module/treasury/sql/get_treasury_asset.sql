SELECT
    asset_address,
    balance,
    reserved_yield,
    available_liquidity,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
FROM treasury_asset_snapshots
WHERE asset_address = $1
