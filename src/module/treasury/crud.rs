use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        auth::error::AuthError,
        treasury::model::{TreasuryAssetRecord, TreasuryStatusRecord},
    },
};

mod sql {
    pub const GET_TREASURY_STATUS: &str = include_str!("sql/get_treasury_status.sql");
    pub const UPSERT_TREASURY_STATUS: &str = include_str!("sql/upsert_treasury_status.sql");
    pub const GET_TREASURY_ASSET: &str = include_str!("sql/get_treasury_asset.sql");
    pub const UPSERT_TREASURY_ASSET: &str = include_str!("sql/upsert_treasury_asset.sql");
}

pub async fn get_treasury_status(
    pool: &DbPool,
    treasury_address: &str,
) -> Result<Option<TreasuryStatusRecord>, AuthError> {
    sqlx::query_as::<_, TreasuryStatusRecord>(sql::GET_TREASURY_STATUS)
        .bind(treasury_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_treasury_status(
    pool: &DbPool,
    treasury_address: &str,
    payment_token_address: &str,
    access_control_address: &str,
    paused: bool,
    total_tracked_balance: &str,
    total_reserved_yield: &str,
    total_reserved_redemptions: &str,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<TreasuryStatusRecord, AuthError> {
    sqlx::query_as::<_, TreasuryStatusRecord>(sql::UPSERT_TREASURY_STATUS)
        .bind(treasury_address)
        .bind(payment_token_address)
        .bind(access_control_address)
        .bind(paused)
        .bind(total_tracked_balance)
        .bind(total_reserved_yield)
        .bind(total_reserved_redemptions)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_treasury_asset(
    pool: &DbPool,
    asset_address: &str,
) -> Result<Option<TreasuryAssetRecord>, AuthError> {
    sqlx::query_as::<_, TreasuryAssetRecord>(sql::GET_TREASURY_ASSET)
        .bind(asset_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_treasury_asset(
    pool: &DbPool,
    asset_address: &str,
    balance: &str,
    reserved_yield: &str,
    reserved_redemptions: &str,
    available_liquidity: &str,
    registered_asset_token: bool,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<TreasuryAssetRecord, AuthError> {
    sqlx::query_as::<_, TreasuryAssetRecord>(sql::UPSERT_TREASURY_ASSET)
        .bind(asset_address)
        .bind(balance)
        .bind(reserved_yield)
        .bind(reserved_redemptions)
        .bind(available_liquidity)
        .bind(registered_asset_token)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}
