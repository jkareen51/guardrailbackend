-- List all users with pending redemptions for a specific asset
-- This helps admins see who needs their redemptions processed
SELECT DISTINCT
    h.user_id,
    h.wallet_address,
    h.asset_address,
    SUM(CASE WHEN h.trade_type = 'redemption' THEN CAST(h.token_amount AS NUMERIC) ELSE 0 END) as total_redemption_requests,
    SUM(CASE WHEN h.trade_type = 'redemption_processed' THEN CAST(h.token_amount AS NUMERIC) ELSE 0 END) as total_processed,
    MAX(h.executed_at) as last_redemption_at
FROM user_trade_history h
WHERE h.asset_address = $1
    AND h.trade_type IN ('redemption', 'redemption_processed')
GROUP BY h.user_id, h.wallet_address, h.asset_address
HAVING SUM(CASE WHEN h.trade_type = 'redemption' THEN CAST(h.token_amount AS NUMERIC) ELSE 0 END) > 
       SUM(CASE WHEN h.trade_type = 'redemption_processed' THEN CAST(h.token_amount AS NUMERIC) ELSE 0 END)
ORDER BY last_redemption_at DESC
