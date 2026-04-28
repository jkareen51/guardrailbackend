use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        asset::model::{AssetRecord, AssetTypeRecord},
        auth::error::AuthError,
    },
};

mod sql {
    pub const GET_ASSET_TYPE: &str = include_str!("sql/get_asset_type.sql");
    pub const LIST_ASSET_TYPES: &str = include_str!("sql/list_asset_types.sql");
    pub const UPSERT_ASSET_TYPE: &str = include_str!("sql/upsert_asset_type.sql");
    pub const GET_ASSET: &str = include_str!("sql/get_asset.sql");
    pub const GET_ASSET_BY_PROPOSAL: &str = include_str!("sql/get_asset_by_proposal.sql");
    pub const LIST_ASSETS: &str = include_str!("sql/list_assets.sql");
    pub const LIST_ASSETS_BY_TYPE: &str = include_str!("sql/list_assets_by_type.sql");
    pub const UPSERT_ASSET: &str = include_str!("sql/upsert_asset.sql");
}

pub async fn get_asset_type(
    pool: &DbPool,
    asset_type_id: &str,
) -> Result<Option<AssetTypeRecord>, AuthError> {
    sqlx::query_as::<_, AssetTypeRecord>(sql::GET_ASSET_TYPE)
        .bind(asset_type_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_asset_types(pool: &DbPool) -> Result<Vec<AssetTypeRecord>, AuthError> {
    sqlx::query_as::<_, AssetTypeRecord>(sql::LIST_ASSET_TYPES)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_asset_type(
    pool: &DbPool,
    asset_type_id: &str,
    asset_type_name: &str,
    implementation_address: &str,
    is_registered: bool,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<AssetTypeRecord, AuthError> {
    sqlx::query_as::<_, AssetTypeRecord>(sql::UPSERT_ASSET_TYPE)
        .bind(asset_type_id)
        .bind(asset_type_name)
        .bind(implementation_address)
        .bind(is_registered)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_asset(
    pool: &DbPool,
    asset_address: &str,
) -> Result<Option<AssetRecord>, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::GET_ASSET)
        .bind(asset_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_asset_by_proposal(
    pool: &DbPool,
    proposal_id: &str,
) -> Result<Option<AssetRecord>, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::GET_ASSET_BY_PROPOSAL)
        .bind(proposal_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_assets(pool: &DbPool) -> Result<Vec<AssetRecord>, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::LIST_ASSETS)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_assets_by_type(
    pool: &DbPool,
    asset_type_id: &str,
) -> Result<Vec<AssetRecord>, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::LIST_ASSETS_BY_TYPE)
        .bind(asset_type_id)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_asset(
    pool: &DbPool,
    asset_address: &str,
    proposal_id: &str,
    asset_type_id: &str,
    name: &str,
    symbol: &str,
    max_supply: &str,
    total_supply: &str,
    asset_state: i32,
    asset_state_label: &str,
    controllable: bool,
    self_service_purchase_enabled: bool,
    price_per_token: &str,
    redemption_price_per_token: &str,
    treasury_address: &str,
    compliance_registry_address: &str,
    payment_token_address: &str,
    metadata_hash: &str,
    holder_count: &str,
    total_pending_redemptions: &str,
    created_by_user_id: Option<Uuid>,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<AssetRecord, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::UPSERT_ASSET)
        .bind(asset_address)
        .bind(proposal_id)
        .bind(asset_type_id)
        .bind(name)
        .bind(symbol)
        .bind(max_supply)
        .bind(total_supply)
        .bind(asset_state)
        .bind(asset_state_label)
        .bind(controllable)
        .bind(self_service_purchase_enabled)
        .bind(price_per_token)
        .bind(redemption_price_per_token)
        .bind(treasury_address)
        .bind(compliance_registry_address)
        .bind(payment_token_address)
        .bind(metadata_hash)
        .bind(holder_count)
        .bind(total_pending_redemptions)
        .bind(created_by_user_id)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}
