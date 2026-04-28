INSERT INTO oracle_trusted_oracles (
    oracle_address,
    is_trusted,
    updated_by_user_id,
    last_tx_hash
) VALUES ($1, $2, $3, $4)
ON CONFLICT (oracle_address) DO UPDATE
SET
    is_trusted = EXCLUDED.is_trusted,
    updated_by_user_id = EXCLUDED.updated_by_user_id,
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, oracle_trusted_oracles.last_tx_hash),
    updated_at = NOW()
RETURNING
    oracle_address,
    is_trusted,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
