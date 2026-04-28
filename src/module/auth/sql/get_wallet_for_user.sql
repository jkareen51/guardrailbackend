SELECT
    wallet_address,
    chain_id,
    account_kind,
    owner_address,
    owner_provider,
    factory_address,
    entry_point_address,
    created_at
FROM wallet_accounts
WHERE user_id = $1
