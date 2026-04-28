use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct AssetTypeRecord {
    pub asset_type_id: String,
    pub asset_type_name: String,
    pub implementation_address: String,
    pub is_registered: bool,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AssetRecord {
    pub asset_address: String,
    pub proposal_id: String,
    pub asset_type_id: String,
    pub name: String,
    pub symbol: String,
    pub max_supply: String,
    pub total_supply: String,
    pub asset_state: i32,
    pub asset_state_label: String,
    pub controllable: bool,
    pub self_service_purchase_enabled: bool,
    pub price_per_token: String,
    pub redemption_price_per_token: String,
    pub treasury_address: String,
    pub compliance_registry_address: String,
    pub payment_token_address: String,
    pub metadata_hash: String,
    pub holder_count: String,
    pub total_pending_redemptions: String,
    pub created_by_user_id: Option<Uuid>,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
