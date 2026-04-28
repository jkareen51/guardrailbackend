INSERT INTO compliance_jurisdiction_restrictions (
    asset_address,
    jurisdiction,
    restricted,
    updated_by_user_id,
    last_tx_hash
)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (asset_address, jurisdiction) DO UPDATE
SET
    restricted = EXCLUDED.restricted,
    updated_by_user_id = COALESCE(
        EXCLUDED.updated_by_user_id,
        compliance_jurisdiction_restrictions.updated_by_user_id
    ),
    last_tx_hash = COALESCE(
        EXCLUDED.last_tx_hash,
        compliance_jurisdiction_restrictions.last_tx_hash
    ),
    updated_at = NOW()
RETURNING
    asset_address,
    jurisdiction,
    restricted,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
