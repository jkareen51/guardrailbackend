SELECT
    oracle_address,
    is_trusted,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
FROM oracle_trusted_oracles
WHERE oracle_address = $1
