INSERT INTO user_trade_history (
    user_id,
    wallet_address,
    asset_address,
    trade_type,
    token_amount,
    payment_amount,
    price_per_token,
    tx_hash,
    executed_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
ON CONFLICT (tx_hash) DO NOTHING
RETURNING
    id,
    user_id,
    wallet_address,
    asset_address,
    trade_type,
    token_amount,
    payment_amount,
    price_per_token,
    tx_hash,
    executed_at,
    created_at
