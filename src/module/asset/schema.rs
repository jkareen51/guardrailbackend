use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    module::{
        asset::model::{AssetCatalogRecord, AssetRecord, AssetTypeRecord},
        compliance::schema::ComplianceAssetRulesResponse,
        oracle::schema::OracleValuationResponse,
        treasury::schema::TreasuryAssetResponse,
    },
    service::chain::bytes32_text_from_hex,
};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ListAssetsQuery {
    pub asset_type_id: Option<String>,
    pub q: Option<String>,
    pub asset_state: Option<String>,
    pub self_service_purchase_enabled: Option<bool>,
    pub featured: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AssetDetailQuery {
    pub wallet_address: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AssetHistoryQuery {
    pub range: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminRegisterAssetTypeRequest {
    pub asset_type_id: String,
    pub asset_type_name: String,
    pub implementation_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminCreateAssetRequest {
    pub proposal_id: String,
    pub asset_type_id: String,
    pub name: String,
    pub symbol: String,
    pub max_supply: String,
    pub subscription_price: String,
    pub redemption_price: String,
    pub self_service_purchase_enabled: bool,
    pub metadata_hash: Option<String>,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub market_segment: Option<String>,
    #[serde(default)]
    pub suggested_internal_tags: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub featured: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub searchable: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminIssueAssetRequest {
    pub recipient_wallet: String,
    pub amount: String,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminBurnAssetRequest {
    pub from_wallet: String,
    pub amount: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetAssetStateRequest {
    pub state: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetAssetPriceRequest {
    pub value: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetAssetPricingRequest {
    pub subscription_price: String,
    pub redemption_price: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetAssetSelfServicePurchaseRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetAssetMetadataRequest {
    pub metadata_hash: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetAssetCatalogRequest {
    pub slug: String,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub market_segment: Option<String>,
    #[serde(default)]
    pub suggested_internal_tags: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub featured: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub searchable: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetAssetComplianceRegistryRequest {
    pub compliance_registry_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetAssetTreasuryRequest {
    pub treasury_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminControllerTransferRequest {
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount: String,
    pub data: Option<String>,
    pub operator_data: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminProcessRedemptionRequest {
    pub investor_wallet: String,
    pub amount: String,
    pub recipient_wallet: String,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssetPreviewRequest {
    pub token_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetCheckTransferRequest {
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount: String,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GaslessApprovePaymentTokenRequest {
    pub amount: String,
}

#[derive(Debug, Deserialize)]
pub struct GaslessPurchaseAssetRequest {
    pub token_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct GaslessClaimYieldRequest {
    pub recipient_wallet: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GaslessRedeemAssetRequest {
    pub amount: String,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GaslessCancelRedemptionRequest {
    pub amount: String,
}

#[derive(Debug, Serialize)]
pub struct AssetFactoryStatusResponse {
    pub factory_address: String,
    pub access_control_address: String,
    pub compliance_registry_address: String,
    pub treasury_address: String,
    pub paused: bool,
    pub total_assets_created: String,
}

#[derive(Debug, Serialize)]
pub struct AssetTypeResponse {
    pub asset_type_id: String,
    pub asset_type_id_text: Option<String>,
    pub asset_type_name: String,
    pub implementation_address: String,
    pub is_registered: bool,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AssetListResponse {
    pub assets: Vec<AssetResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct AssetTypeListResponse {
    pub asset_types: Vec<AssetTypeResponse>,
}

#[derive(Debug, Serialize)]
pub struct AssetResponse {
    pub asset_address: String,
    pub proposal_id: String,
    pub asset_type_id: String,
    pub asset_type_id_text: Option<String>,
    pub asset_type_name: Option<String>,
    pub slug: Option<String>,
    pub name: String,
    pub symbol: String,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub market_segment: Option<String>,
    pub suggested_internal_tags: Vec<String>,
    pub sources: Vec<String>,
    pub featured: bool,
    pub visible: bool,
    pub searchable: bool,
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
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AssetHolderStateResponse {
    pub asset_address: String,
    pub wallet_address: String,
    pub balance: String,
    pub claimable_yield: String,
    pub accumulative_yield: String,
    pub pending_redemption: String,
    pub locked_balance: String,
    pub unlocked_balance: String,
    pub payment_token_balance: String,
    pub payment_token_allowance_to_treasury: String,
}

#[derive(Debug, Serialize)]
pub struct AssetPreviewResponse {
    pub asset_address: String,
    pub token_amount: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct AssetTransferCheckResponse {
    pub status_code: String,
    pub reason_code: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AssetDetailResponse {
    pub asset: AssetResponse,
    pub treasury: Option<TreasuryAssetResponse>,
    pub compliance_rules: Option<ComplianceAssetRulesResponse>,
    pub valuation: Option<OracleValuationResponse>,
    pub holder: Option<AssetHolderStateResponse>,
    pub unavailable_sections: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AssetHistoryCandleResponse {
    pub timestamp: i64,
    pub value: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
}

#[derive(Debug, Serialize)]
pub struct AssetHistoryResponse {
    pub asset_address: String,
    pub range: String,
    pub interval: String,
    pub last_updated_at: Option<i64>,
    pub primary_market_price: Vec<AssetHistoryCandleResponse>,
    pub underlying_market_price: Vec<AssetHistoryCandleResponse>,
}

#[derive(Debug, Serialize)]
pub struct AssetTypeWriteResponse {
    pub tx_hash: String,
    pub asset_type: AssetTypeResponse,
}

#[derive(Debug, Serialize)]
pub struct AssetFactoryWriteResponse {
    pub tx_hash: String,
    pub factory: AssetFactoryStatusResponse,
}

#[derive(Debug, Serialize)]
pub struct AssetWriteResponse {
    pub tx_hash: String,
    pub asset: AssetResponse,
}

#[derive(Debug, Serialize)]
pub struct AssetArchiveWriteResponse {
    pub state_tx_hash: Option<String>,
    pub self_service_purchase_tx_hash: Option<String>,
    pub asset: AssetResponse,
}

#[derive(Debug, Serialize)]
pub struct AssetCatalogWriteResponse {
    pub asset: AssetResponse,
}

#[derive(Debug, Serialize)]
pub struct GaslessAssetActionResponse {
    pub tx_hash: String,
    pub asset: AssetResponse,
    pub holder: AssetHolderStateResponse,
}

impl From<AssetTypeRecord> for AssetTypeResponse {
    fn from(record: AssetTypeRecord) -> Self {
        Self {
            asset_type_id_text: bytes32_text_from_hex(&record.asset_type_id),
            asset_type_id: record.asset_type_id,
            asset_type_name: record.asset_type_name,
            implementation_address: record.implementation_address,
            is_registered: record.is_registered,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}

impl From<AssetRecord> for AssetResponse {
    fn from(record: AssetRecord) -> Self {
        Self {
            asset_address: record.asset_address,
            proposal_id: record.proposal_id,
            asset_type_id_text: bytes32_text_from_hex(&record.asset_type_id),
            asset_type_id: record.asset_type_id,
            asset_type_name: record.asset_type_name,
            slug: record
                .slug
                .or_else(|| Some(default_asset_slug(&record.name))),
            name: record.name,
            symbol: record.symbol,
            image_url: record.image_url,
            summary: record.summary,
            market_segment: record.market_segment,
            suggested_internal_tags: record.suggested_internal_tags,
            sources: record.sources,
            featured: record.featured,
            visible: record.visible,
            searchable: record.searchable,
            max_supply: record.max_supply,
            total_supply: record.total_supply,
            asset_state: record.asset_state,
            asset_state_label: record.asset_state_label,
            controllable: record.controllable,
            self_service_purchase_enabled: record.self_service_purchase_enabled,
            price_per_token: record.price_per_token,
            redemption_price_per_token: record.redemption_price_per_token,
            treasury_address: record.treasury_address,
            compliance_registry_address: record.compliance_registry_address,
            payment_token_address: record.payment_token_address,
            metadata_hash: record.metadata_hash,
            holder_count: record.holder_count,
            total_pending_redemptions: record.total_pending_redemptions,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}

impl AssetCatalogWriteResponse {
    pub fn from_record(record: AssetRecord) -> Self {
        Self {
            asset: AssetResponse::from(record),
        }
    }
}

impl AssetListResponse {
    pub fn new(assets: Vec<AssetResponse>, limit: i64, offset: i64) -> Self {
        Self {
            assets,
            limit,
            offset,
        }
    }
}

impl From<AssetCatalogRecord> for AdminSetAssetCatalogRequest {
    fn from(record: AssetCatalogRecord) -> Self {
        Self {
            slug: record.slug,
            image_url: record.image_url,
            summary: record.summary,
            market_segment: record.market_segment,
            suggested_internal_tags: record.suggested_internal_tags,
            sources: record.sources,
            featured: record.featured,
            visible: record.visible,
            searchable: record.searchable,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_asset_slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut previous_was_hyphen = false;

    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_was_hyphen = false;
            continue;
        }

        if !previous_was_hyphen {
            slug.push('-');
            previous_was_hyphen = true;
        }
    }

    let normalized = slug.trim_matches('-').to_owned();
    if normalized.is_empty() {
        "asset".to_owned()
    } else {
        normalized
    }
}
