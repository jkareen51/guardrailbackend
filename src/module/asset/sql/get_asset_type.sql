SELECT
    asset_type_id,
    asset_type_name,
    implementation_address,
    is_registered,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
FROM asset_types
WHERE asset_type_id = $1
