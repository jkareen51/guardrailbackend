SELECT
    asset_address,
    jurisdiction,
    restricted,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
FROM compliance_jurisdiction_restrictions
WHERE asset_address = $1
  AND jurisdiction = $2
