use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::module::treasury::model::{TreasuryAssetRecord, TreasuryStatusRecord};

#[derive(Debug, Deserialize, Clone)]
pub struct AdminApproveTreasuryPaymentTokenRequest {
    pub amount: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminDepositAssetLiquidityRequest {
    pub asset_address: String,
    pub amount: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminReleaseCapitalRequest {
    pub asset_address: String,
    pub amount: String,
    pub recipient_wallet: String,
    pub reference_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminDepositYieldRequest {
    pub asset_address: String,
    pub amount: String,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminEmergencyWithdrawRequest {
    pub token_address: String,
    pub amount: String,
    pub recipient_wallet: String,
}

#[derive(Debug, Serialize)]
pub struct TreasuryStatusResponse {
    pub treasury_address: String,
    pub payment_token_address: String,
    pub access_control_address: String,
    pub paused: bool,
    pub total_tracked_balance: String,
    pub total_reserved_yield: String,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TreasuryAssetResponse {
    pub asset_address: String,
    pub balance: String,
    pub reserved_yield: String,
    pub available_liquidity: String,
    pub last_tx_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TreasuryStatusWriteResponse {
    pub tx_hash: String,
    pub treasury: TreasuryStatusResponse,
}

#[derive(Debug, Serialize)]
pub struct TreasuryAssetWriteResponse {
    pub tx_hash: String,
    pub treasury: TreasuryStatusResponse,
    pub asset: TreasuryAssetResponse,
}

#[derive(Debug, Serialize)]
pub struct TreasuryPaymentTokenApprovalResponse {
    pub tx_hash: String,
    pub payment_token_address: String,
    pub treasury_address: String,
    pub approved_amount: String,
}

impl From<TreasuryStatusRecord> for TreasuryStatusResponse {
    fn from(record: TreasuryStatusRecord) -> Self {
        Self {
            treasury_address: record.treasury_address,
            payment_token_address: record.payment_token_address,
            access_control_address: record.access_control_address,
            paused: record.paused,
            total_tracked_balance: record.total_tracked_balance,
            total_reserved_yield: record.total_reserved_yield,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}

impl From<TreasuryAssetRecord> for TreasuryAssetResponse {
    fn from(record: TreasuryAssetRecord) -> Self {
        Self {
            asset_address: record.asset_address,
            balance: record.balance,
            reserved_yield: record.reserved_yield,
            available_liquidity: record.available_liquidity,
            last_tx_hash: record.last_tx_hash,
            updated_at: record.updated_at,
        }
    }
}
