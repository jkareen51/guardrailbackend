SELECT
    asset_address,
    asset_value,
    nav_per_token,
    onchain_updated_at,
    reference_id,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
FROM oracle_valuations
WHERE asset_address = $1
