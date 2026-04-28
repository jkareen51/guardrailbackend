use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct TreasuryStatusRecord {
    pub treasury_address: String,
    pub payment_token_address: String,
    pub access_control_address: String,
    pub paused: bool,
    pub total_tracked_balance: String,
    pub total_reserved_yield: String,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct TreasuryAssetRecord {
    pub asset_address: String,
    pub balance: String,
    pub reserved_yield: String,
    pub available_liquidity: String,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
