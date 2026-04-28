use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct OracleTrustedOracleRecord {
    pub oracle_address: String,
    pub is_trusted: bool,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct OracleValuationRecord {
    pub asset_address: String,
    pub asset_value: String,
    pub nav_per_token: String,
    pub onchain_updated_at: i64,
    pub reference_id: String,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct OracleDocumentRecord {
    pub asset_address: String,
    pub document_type: String,
    pub document_hash: String,
    pub reference_id: String,
    pub updated_by_user_id: Option<Uuid>,
    pub last_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
