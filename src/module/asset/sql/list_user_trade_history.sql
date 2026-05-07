SELECT
    h.id,
    h.user_id,
    h.wallet_address,
    h.asset_address,
    h.trade_type,
    h.token_amount,
    h.payment_amount,
    h.price_per_token,
    h.tx_hash,
    h.executed_at,
    h.created_at,
    a.name as asset_name,
    a.symbol as asset_symbol,
    c.image_url as asset_image_url
FROM user_trade_history h
JOIN assets a ON h.asset_address = a.asset_address
LEFT JOIN asset_catalog_entries c ON h.asset_address = c.asset_address
WHERE h.user_id = $1
ORDER BY h.executed_at DESC
LIMIT $2
OFFSET $3
