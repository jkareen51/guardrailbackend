SELECT
    asset_address,
    transfers_enabled,
    subscriptions_enabled,
    redemptions_enabled,
    requires_accreditation,
    min_investment,
    max_investor_balance,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
FROM compliance_asset_rules
WHERE asset_address = $1
