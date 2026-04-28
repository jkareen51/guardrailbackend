use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub const ACCOUNT_KIND_EXTERNAL_EOA: &str = "external_eoa";
pub const ACCOUNT_KIND_SMART_ACCOUNT: &str = "smart_account";
pub const OWNER_PROVIDER_LOCAL: &str = "local";

#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct WalletRecord {
    pub wallet_address: String,
    pub chain_id: i64,
    pub account_kind: String,
    pub owner_address: Option<String>,
    pub owner_provider: Option<String>,
    pub factory_address: Option<String>,
    pub entry_point_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SmartAccountSignerRecord {
    pub wallet_address: String,
    pub owner_address: String,
    pub owner_provider: String,
    pub owner_ref: String,
    pub factory_address: String,
    pub entry_point_address: String,
    pub owner_encrypted_private_key: String,
    pub owner_encryption_nonce: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserProfileRecord {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub wallet_address: Option<String>,
    pub wallet_chain_id: Option<i64>,
    pub wallet_account_kind: Option<String>,
    pub wallet_owner_address: Option<String>,
    pub wallet_owner_provider: Option<String>,
    pub wallet_factory_address: Option<String>,
    pub wallet_entry_point_address: Option<String>,
    pub wallet_created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct NewWalletRecord {
    pub wallet_address: String,
    pub chain_id: i64,
    pub account_kind: String,
    pub owner_address: Option<String>,
    pub owner_provider: Option<String>,
    pub owner_ref: Option<String>,
    pub factory_address: Option<String>,
    pub entry_point_address: Option<String>,
    pub owner_encrypted_private_key: Option<String>,
    pub owner_encryption_nonce: Option<String>,
    pub owner_key_version: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct WalletChallengeRecord {
    pub id: Uuid,
    pub wallet_address: String,
    pub chain_id: i64,
    pub nonce: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct VerifiedGoogleToken {
    pub google_sub: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl UserProfileRecord {
    pub fn into_parts(self) -> (UserRecord, Option<WalletRecord>) {
        let user = UserRecord {
            id: self.id,
            email: self.email,
            username: self.username,
            display_name: self.display_name,
            avatar_url: self.avatar_url,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };

        let wallet = match (
            self.wallet_address,
            self.wallet_chain_id,
            self.wallet_account_kind,
            self.wallet_owner_address,
            self.wallet_owner_provider,
            self.wallet_factory_address,
            self.wallet_entry_point_address,
            self.wallet_created_at,
        ) {
            (
                Some(wallet_address),
                Some(chain_id),
                Some(account_kind),
                owner_address,
                owner_provider,
                factory_address,
                entry_point_address,
                Some(created_at),
            ) => Some(WalletRecord {
                wallet_address,
                chain_id,
                account_kind,
                owner_address,
                owner_provider,
                factory_address,
                entry_point_address,
                created_at,
            }),
            _ => None,
        };

        (user, wallet)
    }
}

impl NewWalletRecord {
    pub fn external_eoa(wallet_address: String, chain_id: i64) -> Self {
        Self {
            wallet_address,
            chain_id,
            account_kind: ACCOUNT_KIND_EXTERNAL_EOA.to_owned(),
            owner_address: None,
            owner_provider: None,
            owner_ref: None,
            factory_address: None,
            entry_point_address: None,
            owner_encrypted_private_key: None,
            owner_encryption_nonce: None,
            owner_key_version: None,
        }
    }

    pub fn smart_account(
        wallet_address: String,
        chain_id: i64,
        owner_address: String,
        owner_provider: String,
        owner_ref: String,
        factory_address: String,
        entry_point_address: String,
        owner_encrypted_private_key: String,
        owner_encryption_nonce: String,
        owner_key_version: i32,
    ) -> Self {
        Self {
            wallet_address,
            chain_id,
            account_kind: ACCOUNT_KIND_SMART_ACCOUNT.to_owned(),
            owner_address: Some(owner_address),
            owner_provider: Some(owner_provider),
            owner_ref: Some(owner_ref),
            factory_address: Some(factory_address),
            entry_point_address: Some(entry_point_address),
            owner_encrypted_private_key: Some(owner_encrypted_private_key),
            owner_encryption_nonce: Some(owner_encryption_nonce),
            owner_key_version: Some(owner_key_version),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub email: Option<String>,
}
