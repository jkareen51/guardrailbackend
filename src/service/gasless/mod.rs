use anyhow::{Result, anyhow};
use ethers_core::types::Address;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::auth::{crud, error::AuthError},
    service::aa::{self, SmartAccountCall, SmartAccountSignerContext},
};

pub async fn submit_user_calls(
    state: &AppState,
    user_id: Uuid,
    calls: Vec<SmartAccountCall>,
) -> Result<String, AuthError> {
    if calls.is_empty() {
        return Err(AuthError::bad_request(
            "gasless execution requires at least one contract call",
        ));
    }

    let wallet = crud::get_smart_account_signer_for_user(&state.db, user_id)
        .await?
        .ok_or_else(|| {
            AuthError::forbidden("gasless execution requires a smart-account user wallet")
        })?;

    let result = aa::submit_calls(
        &state.env,
        &state.http_client,
        &SmartAccountSignerContext {
            wallet_address: wallet.wallet_address,
            owner_address: wallet.owner_address,
            owner_provider: wallet.owner_provider,
            owner_ref: wallet.owner_ref,
            factory_address: wallet.factory_address,
            entry_point_address: wallet.entry_point_address,
            owner_encrypted_private_key: wallet.owner_encrypted_private_key,
            owner_encryption_nonce: wallet.owner_encryption_nonce,
        },
        &calls,
    )
    .await
    .map_err(|error| AuthError::internal("failed to submit gasless smart-account calls", error))?;

    Ok(result.tx_hash)
}

pub fn target_call(target: Address, data: ethers_core::types::Bytes) -> Result<SmartAccountCall> {
    if data.is_empty() {
        return Err(anyhow!("missing calldata for gasless smart-account call"));
    }

    Ok(SmartAccountCall { target, data })
}
