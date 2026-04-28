use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        auth::error::AuthError,
        compliance::model::{
            ComplianceAssetRulesRecord, ComplianceInvestorRecord,
            ComplianceJurisdictionRestrictionRecord,
        },
    },
};

mod sql {
    pub const GET_INVESTOR: &str = include_str!("sql/get_investor.sql");
    pub const UPSERT_INVESTOR: &str = include_str!("sql/upsert_investor.sql");
    pub const GET_ASSET_RULES: &str = include_str!("sql/get_asset_rules.sql");
    pub const UPSERT_ASSET_RULES: &str = include_str!("sql/upsert_asset_rules.sql");
    pub const GET_JURISDICTION_RESTRICTION: &str =
        include_str!("sql/get_jurisdiction_restriction.sql");
    pub const UPSERT_JURISDICTION_RESTRICTION: &str =
        include_str!("sql/upsert_jurisdiction_restriction.sql");
}

pub async fn get_investor(
    pool: &DbPool,
    wallet_address: &str,
) -> Result<Option<ComplianceInvestorRecord>, AuthError> {
    sqlx::query_as::<_, ComplianceInvestorRecord>(sql::GET_INVESTOR)
        .bind(wallet_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_investor(
    pool: &DbPool,
    wallet_address: &str,
    is_verified: bool,
    is_accredited: bool,
    is_frozen: bool,
    valid_until: i64,
    jurisdiction: &str,
    external_ref: &str,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<ComplianceInvestorRecord, AuthError> {
    sqlx::query_as::<_, ComplianceInvestorRecord>(sql::UPSERT_INVESTOR)
        .bind(wallet_address)
        .bind(is_verified)
        .bind(is_accredited)
        .bind(is_frozen)
        .bind(valid_until)
        .bind(jurisdiction)
        .bind(external_ref)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_asset_rules(
    pool: &DbPool,
    asset_address: &str,
) -> Result<Option<ComplianceAssetRulesRecord>, AuthError> {
    sqlx::query_as::<_, ComplianceAssetRulesRecord>(sql::GET_ASSET_RULES)
        .bind(asset_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_asset_rules(
    pool: &DbPool,
    asset_address: &str,
    transfers_enabled: bool,
    subscriptions_enabled: bool,
    redemptions_enabled: bool,
    requires_accreditation: bool,
    min_investment: &str,
    max_investor_balance: &str,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<ComplianceAssetRulesRecord, AuthError> {
    sqlx::query_as::<_, ComplianceAssetRulesRecord>(sql::UPSERT_ASSET_RULES)
        .bind(asset_address)
        .bind(transfers_enabled)
        .bind(subscriptions_enabled)
        .bind(redemptions_enabled)
        .bind(requires_accreditation)
        .bind(min_investment)
        .bind(max_investor_balance)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_jurisdiction_restriction(
    pool: &DbPool,
    asset_address: &str,
    jurisdiction: &str,
) -> Result<Option<ComplianceJurisdictionRestrictionRecord>, AuthError> {
    sqlx::query_as::<_, ComplianceJurisdictionRestrictionRecord>(sql::GET_JURISDICTION_RESTRICTION)
        .bind(asset_address)
        .bind(jurisdiction)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_jurisdiction_restriction(
    pool: &DbPool,
    asset_address: &str,
    jurisdiction: &str,
    restricted: bool,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<ComplianceJurisdictionRestrictionRecord, AuthError> {
    sqlx::query_as::<_, ComplianceJurisdictionRestrictionRecord>(
        sql::UPSERT_JURISDICTION_RESTRICTION,
    )
    .bind(asset_address)
    .bind(jurisdiction)
    .bind(restricted)
    .bind(updated_by_user_id)
    .bind(last_tx_hash)
    .fetch_one(pool)
    .await
    .map_err(AuthError::from)
}
