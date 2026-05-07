SELECT
    treasury_address,
    payment_token_address,
    access_control_address,
    paused,
    total_tracked_balance,
    total_reserved_yield,
    total_reserved_redemptions,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
FROM treasury_status_snapshots
WHERE treasury_address = $1
