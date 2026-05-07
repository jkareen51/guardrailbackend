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
    pub asset_type_name: Option<String>,
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
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub market_segment: Option<String>,
    pub suggested_internal_tags: Vec<String>,
    pub sources: Vec<String>,
    pub featured: bool,
    pub visible: bool,
    pub searchable: bool,
    pub holder_count: String,
    pub total_pending_redemptions: String,
    pub created_by_user_id: Option<Uuid>,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AssetCatalogRecord {
    pub asset_address: String,
    pub slug: String,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub market_segment: Option<String>,
    pub suggested_internal_tags: Vec<String>,
    pub sources: Vec<String>,
    pub featured: bool,
    pub visible: bool,
    pub searchable: bool,
    pub created_by_user_id: Option<Uuid>,
    pub updated_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AssetPriceHistoryRecord {
    pub asset_address: String,
    pub price_per_token: String,
    pub redemption_price_per_token: String,
    pub source: String,
    pub tx_hash: Option<String>,
    pub created_by_user_id: Option<Uuid>,
    pub observed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AssetTagCountRecord {
    pub slug: String,
    pub asset_count: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserTradeHistoryRecord {
    pub id: i64,
    pub user_id: Uuid,
    pub wallet_address: String,
    pub asset_address: String,
    pub trade_type: String,
    pub token_amount: String,
    pub payment_amount: String,
    pub price_per_token: String,
    pub tx_hash: String,
    pub executed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub asset_name: Option<String>,
    pub asset_symbol: Option<String>,
    pub asset_image_url: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PendingRedemptionRecord {
    pub user_id: Uuid,
    pub wallet_address: String,
    pub asset_address: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub last_redemption_at: DateTime<Utc>,
    pub asset_name: String,
    pub asset_symbol: String,
    pub asset_image_url: Option<String>,
}
