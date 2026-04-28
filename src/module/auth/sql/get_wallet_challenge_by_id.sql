SELECT id, wallet_address, chain_id, nonce, message, expires_at, consumed_at, created_at
FROM wallet_challenges
WHERE id = $1
