INSERT INTO compliance_asset_rules (
    asset_address,
    transfers_enabled,
    subscriptions_enabled,
    redemptions_enabled,
    requires_accreditation,
    min_investment,
    max_investor_balance,
    updated_by_user_id,
    last_tx_hash
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT (asset_address) DO UPDATE
SET
    transfers_enabled = EXCLUDED.transfers_enabled,
    subscriptions_enabled = EXCLUDED.subscriptions_enabled,
    redemptions_enabled = EXCLUDED.redemptions_enabled,
    requires_accreditation = EXCLUDED.requires_accreditation,
    min_investment = EXCLUDED.min_investment,
    max_investor_balance = EXCLUDED.max_investor_balance,
    updated_by_user_id = COALESCE(EXCLUDED.updated_by_user_id, compliance_asset_rules.updated_by_user_id),
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, compliance_asset_rules.last_tx_hash),
    updated_at = NOW()
RETURNING
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
