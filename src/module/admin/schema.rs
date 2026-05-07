use serde::{Deserialize, Serialize};

use crate::module::{
    admin::model::{AdminProfile, AdminUploadAssetRecord},
    auth::schema::{
        AuthResponse, UserResponse, WalletChallengeRequest, WalletChallengeResponse,
        WalletConnectRequest,
    },
};

pub type AdminWalletChallengeRequest = WalletChallengeRequest;
pub type AdminWalletChallengeResponse = WalletChallengeResponse;
pub type AdminWalletConnectRequest = WalletConnectRequest;
pub type AdminAuthResponse = AuthResponse;

#[derive(Debug, Serialize)]
pub struct AdminMeResponse {
    pub user: UserResponse,
    pub monad_chain_id: i64,
}

impl AdminMeResponse {
    pub fn from_profile(profile: AdminProfile, monad_chain_id: i64) -> Self {
        Self {
            user: UserResponse::from_parts(profile.user, Some(profile.wallet)),
            monad_chain_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AdminImageUploadResponse {
    pub asset: AdminImageAssetResponse,
}

#[derive(Debug, Serialize)]
pub struct AdminImageAssetResponse {
    pub id: String,
    pub storage_provider: String,
    pub bucket_name: String,
    pub scope: String,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub cid: String,
    pub ipfs_url: String,
    pub gateway_url: String,
    pub created_at: String,
}

impl AdminImageUploadResponse {
    pub fn from_record(record: AdminUploadAssetRecord) -> Self {
        Self {
            asset: AdminImageAssetResponse {
                id: record.id.to_string(),
                storage_provider: record.storage_provider,
                bucket_name: record.bucket_name,
                scope: record.scope,
                file_name: record.file_name,
                content_type: record.content_type,
                size_bytes: record.size_bytes,
                cid: record.cid,
                ipfs_url: record.ipfs_url,
                gateway_url: record.gateway_url,
                created_at: record.created_at.to_rfc3339(),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AdminAccessControlRoleWriteRequest {
    pub role: String,
    pub account_address: String,
}

#[derive(Debug, Serialize)]
pub struct AdminAccessControlRoleSummaryResponse {
    pub role: String,
    pub role_hex: String,
    pub admin_role: String,
    pub admin_role_hex: String,
}

#[derive(Debug, Serialize)]
pub struct AdminAccessControlOverviewResponse {
    pub access_control_address: String,
    pub roles: Vec<AdminAccessControlRoleSummaryResponse>,
}

#[derive(Debug, Serialize)]
pub struct AdminAccessControlRoleMembershipResponse {
    pub access_control_address: String,
    pub account_address: String,
    pub role: String,
    pub role_hex: String,
    pub has_role: bool,
    pub admin_role: String,
    pub admin_role_hex: String,
}

#[derive(Debug, Serialize)]
pub struct AdminAccessControlRoleWriteResponse {
    pub tx_hash: String,
    pub action: String,
    pub membership: AdminAccessControlRoleMembershipResponse,
}

#[derive(Debug, Deserialize)]
pub struct AdminMultiSigProposalRequest {
    pub target: String,
    pub data: String,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminMultiSigSignerWriteRequest {
    pub signer_address: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminMultiSigQuorumWriteRequest {
    pub quorum: String,
}

#[derive(Debug, Serialize)]
pub struct AdminMultiSigOverviewResponse {
    pub multisig_address: String,
    pub signers: Vec<String>,
    pub quorum: String,
    pub proposal_count: String,
    pub timelock_duration: String,
    pub proposal_expiry: String,
    pub min_timelock: String,
}

#[derive(Debug, Serialize)]
pub struct AdminMultiSigProposalResponse {
    pub multisig_address: String,
    pub proposal_id: String,
    pub proposal_hash: String,
    pub target: String,
    pub data: String,
    pub value: String,
    pub signatures_count: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub timelock_until: i64,
    pub executed: bool,
    pub cancelled: bool,
    pub proposer: String,
    pub ready_to_execute: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminMultiSigProposalSignatureResponse {
    pub multisig_address: String,
    pub proposal_id: String,
    pub signer_address: String,
    pub has_signed: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminMultiSigProposalWriteResponse {
    pub tx_hash: String,
    pub proposal: AdminMultiSigProposalResponse,
}
