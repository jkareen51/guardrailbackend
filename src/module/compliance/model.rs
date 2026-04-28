use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct ComplianceInvestorRecord {
    pub wallet_address: String,
    pub is_verified: bool,
    pub is_accredited: bool,
    pub is_frozen: bool,
    pub valid_until: i64,
    pub jurisdiction: String,
    pub external_ref: String,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ComplianceAssetRulesRecord {
    pub asset_address: String,
    pub transfers_enabled: bool,
    pub subscriptions_enabled: bool,
    pub redemptions_enabled: bool,
    pub requires_accreditation: bool,
    pub min_investment: String,
    pub max_investor_balance: String,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ComplianceJurisdictionRestrictionRecord {
    pub asset_address: String,
    pub jurisdiction: String,
    pub restricted: bool,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
