INSERT INTO treasury_status_snapshots (
    treasury_address,
    payment_token_address,
    access_control_address,
    paused,
    total_tracked_balance,
    total_reserved_yield,
    updated_by_user_id,
    last_tx_hash
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (treasury_address) DO UPDATE
SET
    payment_token_address = EXCLUDED.payment_token_address,
    access_control_address = EXCLUDED.access_control_address,
    paused = EXCLUDED.paused,
    total_tracked_balance = EXCLUDED.total_tracked_balance,
    total_reserved_yield = EXCLUDED.total_reserved_yield,
    updated_by_user_id = EXCLUDED.updated_by_user_id,
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, treasury_status_snapshots.last_tx_hash),
    updated_at = NOW()
RETURNING
    treasury_address,
    payment_token_address,
    access_control_address,
    paused,
    total_tracked_balance,
    total_reserved_yield,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
