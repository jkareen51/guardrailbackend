use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    module::oracle::model::{
        OracleDocumentRecord, OracleTrustedOracleRecord, OracleValuationRecord,
    },
    service::chain::bytes32_text_from_hex,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetTrustedOracleRequest {
    pub trusted: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSubmitValuationRequest {
    pub asset_address: String,
    pub asset_value: String,
    pub nav_per_token: String,
    pub reference_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSubmitValuationAndSyncPricingRequest {
    pub asset_address: String,
    pub asset_value: String,
    pub nav_per_token: String,
    pub subscription_price: String,
    pub redemption_price: String,
    pub reference_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminAnchorDocumentRequest {
    pub document_hash: String,
    pub reference_id: String,
}

#[derive(Debug, Serialize)]
pub struct OracleTrustedOracleResponse {
    pub oracle_address: String,
    pub is_trusted: bool,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OracleValuationResponse {
    pub asset_address: String,
    pub asset_value: String,
    pub nav_per_token: String,
    pub onchain_updated_at: i64,
    pub reference_id: String,
    pub reference_id_text: Option<String>,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OracleValuationFreshnessResponse {
    pub asset_address: String,
    pub is_fresh: bool,
    pub max_age_seconds: i64,
    pub last_updated_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct OracleDocumentResponse {
    pub asset_address: String,
    pub document_type: String,
    pub document_type_text: Option<String>,
    pub document_hash: String,
    pub reference_id: String,
    pub reference_id_text: Option<String>,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OracleTrustedOracleWriteResponse {
    pub tx_hash: String,
    pub trusted_oracle: OracleTrustedOracleResponse,
}

#[derive(Debug, Serialize)]
pub struct OracleValuationWriteResponse {
    pub tx_hash: String,
    pub valuation: OracleValuationResponse,
}

#[derive(Debug, Serialize)]
pub struct OracleDocumentWriteResponse {
    pub tx_hash: String,
    pub document: OracleDocumentResponse,
}

impl From<OracleTrustedOracleRecord> for OracleTrustedOracleResponse {
    fn from(record: OracleTrustedOracleRecord) -> Self {
        Self {
            oracle_address: record.oracle_address,
            is_trusted: record.is_trusted,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}

impl From<OracleValuationRecord> for OracleValuationResponse {
    fn from(record: OracleValuationRecord) -> Self {
        Self {
            asset_address: record.asset_address,
            asset_value: record.asset_value,
            nav_per_token: record.nav_per_token,
            onchain_updated_at: record.onchain_updated_at,
            reference_id_text: bytes32_text_from_hex(&record.reference_id),
            reference_id: record.reference_id,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}

impl From<OracleDocumentRecord> for OracleDocumentResponse {
    fn from(record: OracleDocumentRecord) -> Self {
        Self {
            asset_address: record.asset_address,
            document_type_text: bytes32_text_from_hex(&record.document_type),
            document_type: record.document_type,
            document_hash: record.document_hash,
            reference_id_text: bytes32_text_from_hex(&record.reference_id),
            reference_id: record.reference_id,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}
