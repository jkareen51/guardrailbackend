use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct FaucetUsdcRequest {
    pub amount: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FaucetUsdcBalanceQuery {
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct FaucetUsdcResponse {
    pub token_address: String,
    pub recipient: String,
    pub wallet_account_kind: String,
    pub amount: String,
    pub balance: String,
    pub tx_hash: String,
    pub requested_at: DateTime<Utc>,
    pub next_available_at: DateTime<Utc>,
    pub cooldown_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct FaucetUsdcBalanceResponse {
    pub token_address: String,
    pub address: String,
    pub balance: String,
    pub queried_at: DateTime<Utc>,
}
