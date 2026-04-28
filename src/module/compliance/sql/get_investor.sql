SELECT
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
FROM compliance_investors
WHERE wallet_address = $1
