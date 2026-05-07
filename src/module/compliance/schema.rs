use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::module::compliance::model::{
    ComplianceAssetRulesRecord, ComplianceInvestorRecord, ComplianceJurisdictionRestrictionRecord,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AdminUpsertComplianceInvestorRequest {
    pub is_verified: bool,
    pub is_accredited: bool,
    pub is_frozen: bool,
    pub valid_until: Option<i64>,
    pub jurisdiction: String,
    pub external_ref: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminBatchUpsertComplianceInvestorItem {
    pub wallet_address: String,
    pub is_verified: bool,
    pub is_accredited: bool,
    pub is_frozen: bool,
    pub valid_until: Option<i64>,
    pub jurisdiction: String,
    pub external_ref: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminBatchUpsertComplianceInvestorsRequest {
    pub investors: Vec<AdminBatchUpsertComplianceInvestorItem>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminBatchWhitelistComplianceInvestorsRequest {
    pub wallet_addresses: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetComplianceAssetRulesRequest {
    pub transfers_enabled: bool,
    pub subscriptions_enabled: bool,
    pub redemptions_enabled: bool,
    pub requires_accreditation: bool,
    pub min_investment: String,
    pub max_investor_balance: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetComplianceJurisdictionRestrictionRequest {
    pub restricted: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetComplianceInvestorStatusRequest {
    pub is_accredited: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminSetComplianceAccessControlRequest {
    pub access_control_address: String,
}

#[derive(Debug, Deserialize)]
pub struct ComplianceCheckSubscribeRequest {
    pub asset_address: String,
    pub investor_wallet: String,
    pub amount: String,
    pub resulting_balance: String,
}

#[derive(Debug, Deserialize)]
pub struct ComplianceCheckTransferRequest {
    pub asset_address: String,
    pub from_wallet: String,
    pub to_wallet: String,
    pub amount: String,
    pub receiving_balance: String,
}

#[derive(Debug, Deserialize)]
pub struct ComplianceCheckRedeemRequest {
    pub asset_address: String,
    pub investor_wallet: String,
    pub amount: String,
}

#[derive(Debug, Serialize)]
pub struct ComplianceInvestorResponse {
    pub wallet_address: String,
    pub is_verified: bool,
    pub is_accredited: bool,
    pub is_frozen: bool,
    pub is_whitelisted: bool,
    pub valid_until: Option<i64>,
    pub jurisdiction: String,
    pub jurisdiction_text: Option<String>,
    pub external_ref: String,
    pub external_ref_text: Option<String>,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ComplianceAssetRulesResponse {
    pub asset_address: String,
    pub transfers_enabled: bool,
    pub subscriptions_enabled: bool,
    pub redemptions_enabled: bool,
    pub requires_accreditation: bool,
    pub min_investment: String,
    pub max_investor_balance: String,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ComplianceJurisdictionRestrictionResponse {
    pub asset_address: String,
    pub jurisdiction: String,
    pub jurisdiction_text: Option<String>,
    pub restricted: bool,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminComplianceInvestorUpsertResponse {
    pub tx_hash: String,
    pub investor: ComplianceInvestorResponse,
}

#[derive(Debug, Serialize)]
pub struct AdminComplianceInvestorBatchUpsertResponse {
    pub tx_hash: String,
    pub investors: Vec<ComplianceInvestorResponse>,
}

#[derive(Debug, Serialize)]
pub struct ComplianceAccessControlResponse {
    pub compliance_address: String,
    pub access_control_address: String,
}

#[derive(Debug, Serialize)]
pub struct ComplianceAccessControlWriteResponse {
    pub tx_hash: String,
    pub compliance: ComplianceAccessControlResponse,
}

#[derive(Debug, Serialize)]
pub struct AdminComplianceAssetRulesUpsertResponse {
    pub tx_hash: String,
    pub asset_rules: ComplianceAssetRulesResponse,
}

#[derive(Debug, Serialize)]
pub struct AdminComplianceJurisdictionRestrictionUpsertResponse {
    pub tx_hash: String,
    pub restriction: ComplianceJurisdictionRestrictionResponse,
}

#[derive(Debug, Serialize)]
pub struct ComplianceCheckResponse {
    pub is_valid: bool,
    pub reason: String,
}

impl ComplianceInvestorResponse {
    pub fn from_record(record: ComplianceInvestorRecord) -> Self {
        let now = Utc::now().timestamp();
        let is_whitelisted = record.is_verified
            && !record.is_frozen
            && (record.valid_until == 0 || record.valid_until >= now);

        Self {
            wallet_address: record.wallet_address,
            is_verified: record.is_verified,
            is_accredited: record.is_accredited,
            is_frozen: record.is_frozen,
            is_whitelisted,
            valid_until: (record.valid_until != 0).then_some(record.valid_until),
            jurisdiction_text: bytes32_text_from_hex(&record.jurisdiction),
            jurisdiction: record.jurisdiction,
            external_ref_text: bytes32_text_from_hex(&record.external_ref),
            external_ref: record.external_ref,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}

impl From<ComplianceAssetRulesRecord> for ComplianceAssetRulesResponse {
    fn from(record: ComplianceAssetRulesRecord) -> Self {
        Self {
            asset_address: record.asset_address,
            transfers_enabled: record.transfers_enabled,
            subscriptions_enabled: record.subscriptions_enabled,
            redemptions_enabled: record.redemptions_enabled,
            requires_accreditation: record.requires_accreditation,
            min_investment: record.min_investment,
            max_investor_balance: record.max_investor_balance,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}

impl From<ComplianceJurisdictionRestrictionRecord> for ComplianceJurisdictionRestrictionResponse {
    fn from(record: ComplianceJurisdictionRestrictionRecord) -> Self {
        Self {
            asset_address: record.asset_address,
            jurisdiction_text: bytes32_text_from_hex(&record.jurisdiction),
            jurisdiction: record.jurisdiction,
            restricted: record.restricted,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}

fn bytes32_text_from_hex(raw: &str) -> Option<String> {
    let stripped = raw.strip_prefix("0x").unwrap_or(raw);
    let bytes = hex::decode(stripped).ok()?;
    if bytes.len() != 32 {
        return None;
    }

    let trimmed = bytes
        .into_iter()
        .take_while(|value| *value != 0)
        .collect::<Vec<_>>();
    if trimmed.is_empty() {
        return None;
    }
    if !trimmed
        .iter()
        .all(|value| value.is_ascii_graphic() || *value == b' ')
    {
        return None;
    }

    String::from_utf8(trimmed).ok()
}
