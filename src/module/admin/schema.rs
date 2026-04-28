use serde::Serialize;

use crate::module::{
    admin::model::AdminProfile,
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
