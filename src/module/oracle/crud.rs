use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        auth::error::AuthError,
        oracle::model::{OracleDocumentRecord, OracleTrustedOracleRecord, OracleValuationRecord},
    },
};

mod sql {
    pub const GET_TRUSTED_ORACLE: &str = include_str!("sql/get_trusted_oracle.sql");
    pub const UPSERT_TRUSTED_ORACLE: &str = include_str!("sql/upsert_trusted_oracle.sql");
    pub const GET_VALUATION: &str = include_str!("sql/get_valuation.sql");
    pub const UPSERT_VALUATION: &str = include_str!("sql/upsert_valuation.sql");
    pub const GET_DOCUMENT: &str = include_str!("sql/get_document.sql");
    pub const UPSERT_DOCUMENT: &str = include_str!("sql/upsert_document.sql");
}

pub async fn get_trusted_oracle(
    pool: &DbPool,
    oracle_address: &str,
) -> Result<Option<OracleTrustedOracleRecord>, AuthError> {
    sqlx::query_as::<_, OracleTrustedOracleRecord>(sql::GET_TRUSTED_ORACLE)
        .bind(oracle_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_trusted_oracle(
    pool: &DbPool,
    oracle_address: &str,
    is_trusted: bool,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<OracleTrustedOracleRecord, AuthError> {
    sqlx::query_as::<_, OracleTrustedOracleRecord>(sql::UPSERT_TRUSTED_ORACLE)
        .bind(oracle_address)
        .bind(is_trusted)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_valuation(
    pool: &DbPool,
    asset_address: &str,
) -> Result<Option<OracleValuationRecord>, AuthError> {
    sqlx::query_as::<_, OracleValuationRecord>(sql::GET_VALUATION)
        .bind(asset_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_valuation(
    pool: &DbPool,
    asset_address: &str,
    asset_value: &str,
    nav_per_token: &str,
    onchain_updated_at: i64,
    reference_id: &str,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<OracleValuationRecord, AuthError> {
    sqlx::query_as::<_, OracleValuationRecord>(sql::UPSERT_VALUATION)
        .bind(asset_address)
        .bind(asset_value)
        .bind(nav_per_token)
        .bind(onchain_updated_at)
        .bind(reference_id)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_document(
    pool: &DbPool,
    asset_address: &str,
    document_type: &str,
) -> Result<Option<OracleDocumentRecord>, AuthError> {
    sqlx::query_as::<_, OracleDocumentRecord>(sql::GET_DOCUMENT)
        .bind(asset_address)
        .bind(document_type)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_document(
    pool: &DbPool,
    asset_address: &str,
    document_type: &str,
    document_hash: &str,
    reference_id: &str,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<OracleDocumentRecord, AuthError> {
    sqlx::query_as::<_, OracleDocumentRecord>(sql::UPSERT_DOCUMENT)
        .bind(asset_address)
        .bind(document_type)
        .bind(document_hash)
        .bind(reference_id)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}
