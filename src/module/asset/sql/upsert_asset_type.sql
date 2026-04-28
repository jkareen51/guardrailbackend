INSERT INTO asset_types (
    asset_type_id,
    asset_type_name,
    implementation_address,
    is_registered,
    updated_by_user_id,
    last_tx_hash
) VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (asset_type_id) DO UPDATE
SET
    asset_type_name = EXCLUDED.asset_type_name,
    implementation_address = EXCLUDED.implementation_address,
    is_registered = EXCLUDED.is_registered,
    updated_by_user_id = EXCLUDED.updated_by_user_id,
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, asset_types.last_tx_hash),
    updated_at = NOW()
RETURNING
    asset_type_id,
    asset_type_name,
    implementation_address,
    is_registered,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
