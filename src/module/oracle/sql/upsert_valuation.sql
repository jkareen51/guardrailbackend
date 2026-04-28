INSERT INTO oracle_valuations (
    asset_address,
    asset_value,
    nav_per_token,
    onchain_updated_at,
    reference_id,
    updated_by_user_id,
    last_tx_hash
) VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (asset_address) DO UPDATE
SET
    asset_value = EXCLUDED.asset_value,
    nav_per_token = EXCLUDED.nav_per_token,
    onchain_updated_at = EXCLUDED.onchain_updated_at,
    reference_id = EXCLUDED.reference_id,
    updated_by_user_id = EXCLUDED.updated_by_user_id,
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, oracle_valuations.last_tx_hash),
    updated_at = NOW()
RETURNING
    asset_address,
    asset_value,
    nav_per_token,
    onchain_updated_at,
    reference_id,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
