INSERT INTO compliance_investors (
    wallet_address,
    is_verified,
    is_accredited,
    is_frozen,
    valid_until,
    jurisdiction,
    external_ref,
    updated_by_user_id,
    last_tx_hash
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT (wallet_address) DO UPDATE
SET
    is_verified = EXCLUDED.is_verified,
    is_accredited = EXCLUDED.is_accredited,
    is_frozen = EXCLUDED.is_frozen,
    valid_until = EXCLUDED.valid_until,
    jurisdiction = EXCLUDED.jurisdiction,
    external_ref = EXCLUDED.external_ref,
    updated_by_user_id = COALESCE(EXCLUDED.updated_by_user_id, compliance_investors.updated_by_user_id),
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, compliance_investors.last_tx_hash),
    updated_at = NOW()
RETURNING
    wallet_address,
    is_verified,
    is_accredited,
    is_frozen,
    valid_until,
    jurisdiction,
    external_ref,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
