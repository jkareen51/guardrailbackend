use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::module::auth::model::{UserRecord, WalletRecord};

#[derive(Debug, Deserialize)]
pub struct GoogleSignInRequest {
    pub credential: String,
    pub g_csrf_token: Option<String>,
    pub client_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WalletChallengeRequest {
    pub wallet_address: String,
}

#[derive(Debug, Deserialize)]
pub struct WalletConnectRequest {
    pub challenge_id: Uuid,
    pub signature: String,
    pub username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct WalletChallengeResponse {
    pub challenge_id: Uuid,
    pub message: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub wallet: Option<WalletResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WalletResponse {
    pub wallet_address: String,
    pub chain_id: i64,
    pub account_kind: String,
    pub owner_address: Option<String>,
    pub owner_provider: Option<String>,
    pub factory_address: Option<String>,
    pub entry_point_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl UserResponse {
    pub fn from_parts(user: UserRecord, wallet: Option<WalletRecord>) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            wallet: wallet.map(WalletResponse::from),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

impl From<WalletRecord> for WalletResponse {
    fn from(value: WalletRecord) -> Self {
        Self {
            wallet_address: value.wallet_address,
            chain_id: value.chain_id,
            account_kind: value.account_kind,
            owner_address: value.owner_address,
            owner_provider: value.owner_provider,
            factory_address: value.factory_address,
            entry_point_address: value.entry_point_address,
            created_at: value.created_at,
        }
    }
}
