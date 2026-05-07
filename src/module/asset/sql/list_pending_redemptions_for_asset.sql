-- List all pending redemptions for an asset from trade history
SELECT DISTINCT
    h.user_id,
    h.wallet_address,
    h.asset_address,
    u.email,
    u.display_name,
    MAX(h.executed_at) as last_redemption_at,
    a.name as asset_name,
    a.symbol as asset_symbol,
    c.image_url as asset_image_url
FROM user_trade_history h
JOIN users u ON h.user_id = u.id
JOIN assets a ON h.asset_address = a.asset_address
LEFT JOIN asset_catalog_entries c ON h.asset_address = c.asset_address
WHERE h.asset_address = $1
    AND h.trade_type = 'redemption'
GROUP BY h.user_id, h.wallet_address, h.asset_address, u.email, u.display_name, a.name, a.symbol, c.image_url
ORDER BY last_redemption_at DESC
