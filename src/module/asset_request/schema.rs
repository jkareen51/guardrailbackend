use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    module::{asset::schema::AssetResponse, asset_request::model::AssetRequestRecord},
    service::chain::bytes32_text_from_hex,
};

#[derive(Debug, Deserialize, Clone)]
pub struct CreateAssetRequestRequest {
    pub issuer_name: String,
    pub contact_name: String,
    pub contact_email: String,
    pub issuer_website: Option<String>,
    pub issuer_country: Option<String>,
    pub asset_name: String,
    pub asset_type_id: String,
    pub description: String,
    pub target_raise: Option<String>,
    pub currency: Option<String>,
    pub maturity_date: Option<String>,
    pub expected_yield_bps: Option<i32>,
    pub redemption_summary: Option<String>,
    pub valuation_source: Option<String>,
    #[serde(default)]
    pub document_urls: Vec<String>,
    pub token_symbol: String,
    pub max_supply: String,
    pub subscription_price: String,
    pub redemption_price: String,
    #[serde(default = "default_true")]
    pub self_service_purchase_enabled: bool,
    pub metadata_hash: Option<String>,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub market_segment: Option<String>,
    #[serde(default)]
    pub suggested_internal_tags: Vec<String>,
    #[serde(default)]
    pub source_urls: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ListAssetRequestsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminUpdateAssetRequestStatusRequest {
    pub status: String,
    pub review_notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssetRequestResponse {
    pub id: String,
    pub proposal_id: String,
    pub submitted_by_user_id: String,
    pub issuer_name: String,
    pub contact_name: String,
    pub contact_email: String,
    pub issuer_website: Option<String>,
    pub issuer_country: Option<String>,
    pub asset_name: String,
    pub asset_type_id: String,
    pub asset_type_id_text: Option<String>,
    pub description: String,
    pub target_raise: Option<String>,
    pub currency: Option<String>,
    pub maturity_date: Option<NaiveDate>,
    pub expected_yield_bps: Option<i32>,
    pub redemption_summary: Option<String>,
    pub valuation_source: Option<String>,
    pub document_urls: Vec<String>,
    pub token_symbol: String,
    pub max_supply: String,
    pub subscription_price: String,
    pub redemption_price: String,
    pub self_service_purchase_enabled: bool,
    pub metadata_hash: Option<String>,
    pub metadata_hash_text: Option<String>,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub market_segment: Option<String>,
    pub suggested_internal_tags: Vec<String>,
    pub source_urls: Vec<String>,
    pub status: String,
    pub review_notes: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub deployed_by_user_id: Option<String>,
    pub deployed_at: Option<DateTime<Utc>>,
    pub deployed_asset_address: Option<String>,
    pub deployment_tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AssetRequestListResponse {
    pub asset_requests: Vec<AssetRequestResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct AssetRequestDeployResponse {
    pub tx_hash: Option<String>,
    pub request: AssetRequestResponse,
    pub asset: AssetResponse,
}

impl From<AssetRequestRecord> for AssetRequestResponse {
    fn from(record: AssetRequestRecord) -> Self {
        Self {
            id: record.id.to_string(),
            proposal_id: record.proposal_id.to_string(),
            submitted_by_user_id: record.submitted_by_user_id.to_string(),
            issuer_name: record.issuer_name,
            contact_name: record.contact_name,
            contact_email: record.contact_email,
            issuer_website: record.issuer_website,
            issuer_country: record.issuer_country,
            asset_name: record.asset_name,
            asset_type_id_text: bytes32_text_from_hex(&record.asset_type_id),
            asset_type_id: record.asset_type_id,
            description: record.description,
            target_raise: record.target_raise,
            currency: record.currency,
            maturity_date: record.maturity_date,
            expected_yield_bps: record.expected_yield_bps,
            redemption_summary: record.redemption_summary,
            valuation_source: record.valuation_source,
            document_urls: record.document_urls,
            token_symbol: record.token_symbol,
            max_supply: record.max_supply,
            subscription_price: record.subscription_price,
            redemption_price: record.redemption_price,
            self_service_purchase_enabled: record.self_service_purchase_enabled,
            metadata_hash_text: record
                .metadata_hash
                .as_deref()
                .and_then(bytes32_text_from_hex),
            metadata_hash: record.metadata_hash,
            slug: record.slug,
            image_url: record.image_url,
            market_segment: record.market_segment,
            suggested_internal_tags: record.suggested_internal_tags,
            source_urls: record.source_urls,
            status: record.status,
            review_notes: record.review_notes,
            reviewed_by_user_id: record.reviewed_by_user_id.map(|value| value.to_string()),
            reviewed_at: record.reviewed_at,
            deployed_by_user_id: record.deployed_by_user_id.map(|value| value.to_string()),
            deployed_at: record.deployed_at,
            deployed_asset_address: record.deployed_asset_address,
            deployment_tx_hash: record.deployment_tx_hash,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}
