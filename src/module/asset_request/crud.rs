use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        asset_request::model::{AssetRequestRecord, NewAssetRequestRecord},
        auth::error::AuthError,
    },
};

mod sql {
    pub const GET_ASSET_REQUEST: &str = include_str!("sql/get_asset_request.sql");
    pub const LIST_ASSET_REQUESTS: &str = include_str!("sql/list_asset_requests.sql");
    pub const LIST_ASSET_REQUESTS_FOR_SUBMITTER: &str =
        include_str!("sql/list_asset_requests_for_submitter.sql");
    pub const INSERT_ASSET_REQUEST: &str = include_str!("sql/insert_asset_request.sql");
    pub const UPDATE_ASSET_REQUEST_STATUS: &str =
        include_str!("sql/update_asset_request_status.sql");
    pub const MARK_ASSET_REQUEST_DEPLOYED: &str =
        include_str!("sql/mark_asset_request_deployed.sql");
}

pub async fn get_asset_request(
    pool: &DbPool,
    request_id: Uuid,
) -> Result<Option<AssetRequestRecord>, AuthError> {
    sqlx::query_as::<_, AssetRequestRecord>(sql::GET_ASSET_REQUEST)
        .bind(request_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_asset_requests(
    pool: &DbPool,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AssetRequestRecord>, AuthError> {
    sqlx::query_as::<_, AssetRequestRecord>(sql::LIST_ASSET_REQUESTS)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_asset_requests_for_submitter(
    pool: &DbPool,
    user_id: Uuid,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AssetRequestRecord>, AuthError> {
    sqlx::query_as::<_, AssetRequestRecord>(sql::LIST_ASSET_REQUESTS_FOR_SUBMITTER)
        .bind(user_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn create_asset_request(
    pool: &DbPool,
    record: &NewAssetRequestRecord,
) -> Result<AssetRequestRecord, AuthError> {
    sqlx::query_as::<_, AssetRequestRecord>(sql::INSERT_ASSET_REQUEST)
        .bind(Uuid::new_v4())
        .bind(record.submitted_by_user_id)
        .bind(&record.issuer_name)
        .bind(&record.contact_name)
        .bind(&record.contact_email)
        .bind(record.issuer_website.as_deref())
        .bind(record.issuer_country.as_deref())
        .bind(&record.asset_name)
        .bind(&record.asset_type_id)
        .bind(&record.description)
        .bind(record.target_raise.as_deref())
        .bind(record.currency.as_deref())
        .bind(record.maturity_date)
        .bind(record.expected_yield_bps)
        .bind(record.redemption_summary.as_deref())
        .bind(record.valuation_source.as_deref())
        .bind(&record.document_urls)
        .bind(&record.token_symbol)
        .bind(&record.max_supply)
        .bind(&record.subscription_price)
        .bind(&record.redemption_price)
        .bind(record.self_service_purchase_enabled)
        .bind(record.metadata_hash.as_deref())
        .bind(record.slug.as_deref())
        .bind(record.image_url.as_deref())
        .bind(record.market_segment.as_deref())
        .bind(&record.suggested_internal_tags)
        .bind(&record.source_urls)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn update_asset_request_status(
    pool: &DbPool,
    request_id: Uuid,
    status: &str,
    review_notes: Option<&str>,
    reviewed_by_user_id: Uuid,
    reviewed_at: DateTime<Utc>,
) -> Result<AssetRequestRecord, AuthError> {
    sqlx::query_as::<_, AssetRequestRecord>(sql::UPDATE_ASSET_REQUEST_STATUS)
        .bind(request_id)
        .bind(status)
        .bind(review_notes)
        .bind(reviewed_by_user_id)
        .bind(reviewed_at)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn mark_asset_request_deployed(
    pool: &DbPool,
    request_id: Uuid,
    deployed_asset_address: &str,
    deployment_tx_hash: Option<&str>,
    deployed_by_user_id: Uuid,
    deployed_at: DateTime<Utc>,
) -> Result<AssetRequestRecord, AuthError> {
    sqlx::query_as::<_, AssetRequestRecord>(sql::MARK_ASSET_REQUEST_DEPLOYED)
        .bind(request_id)
        .bind(deployed_asset_address)
        .bind(deployment_tx_hash)
        .bind(deployed_by_user_id)
        .bind(deployed_at)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}
