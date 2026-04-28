use std::{str::FromStr, sync::Arc};

use anyhow::{Context, Result, anyhow};
use ethers_core::types::{Address, Bytes, H256, U256};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Signer};

use crate::{config::environment::Environment, module::auth::error::AuthError, service::rpc};

pub async fn admin_signer(
    env: &Environment,
) -> Result<Arc<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let private_key = env
        .operator_private_key
        .as_deref()
        .ok_or_else(|| AuthError::forbidden("missing OPERATOR_PRIVATE_KEY for admin writes"))?;

    let chain_id = u64::try_from(env.monad_chain_id)
        .map_err(|_| AuthError::bad_request("MONAD_CHAIN_ID must be non-negative"))?;
    let wallet = LocalWallet::from_str(private_key)
        .map_err(|error| AuthError::internal("invalid OPERATOR_PRIVATE_KEY", error))?
        .with_chain_id(chain_id);
    let signer_address = format_address(wallet.address());

    if !env.is_admin_wallet(&signer_address) {
        return Err(AuthError::forbidden(
            "OPERATOR_PRIVATE_KEY address is not listed in ADMIN_WALLET_ADDRESSES",
        ));
    }

    rpc::monad_signer_middleware(env, wallet)
        .await
        .map_err(|error| AuthError::internal("failed to build signer middleware", error))
}

pub async fn wait_for_receipt<P>(
    pending: ethers_providers::PendingTransaction<'_, P>,
) -> Result<String, AuthError>
where
    P: ethers_providers::JsonRpcClient,
{
    let receipt = pending
        .await
        .map_err(|error| {
            AuthError::internal("transaction failed while waiting for receipt", error)
        })?
        .ok_or_else(|| {
            AuthError::internal(
                "transaction dropped before receipt",
                anyhow!("missing receipt"),
            )
        })?;

    Ok(format!("{:#x}", receipt.transaction_hash))
}

pub fn parse_address(raw: &str) -> Result<Address, AuthError> {
    raw.trim()
        .parse::<Address>()
        .map_err(|_| AuthError::bad_request("invalid address"))
}

pub fn parse_contract_address(raw: &str) -> Result<Address> {
    raw.trim()
        .parse::<Address>()
        .with_context(|| format!("invalid contract address `{raw}`"))
}

pub fn format_address(address: Address) -> String {
    format!("{address:#x}")
}

pub fn parse_u256(raw: &str, field_name: &str) -> Result<U256, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    if let Some(stripped) = value.strip_prefix("0x") {
        return U256::from_str_radix(stripped, 16)
            .map_err(|_| AuthError::bad_request(format!("invalid {field_name} hex amount")));
    }

    U256::from_dec_str(value)
        .map_err(|_| AuthError::bad_request(format!("invalid {field_name} amount")))
}

pub fn u256_to_string(value: U256) -> String {
    value.to_string()
}

pub fn parse_bytes32_input(raw: &str, field_name: &str) -> Result<H256, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok(H256::zero());
    }

    if let Some(stripped) = value.strip_prefix("0x") {
        let bytes = hex::decode(stripped)
            .map_err(|_| AuthError::bad_request(format!("invalid {field_name} hex value")))?;
        if bytes.len() != 32 {
            return Err(AuthError::bad_request(format!(
                "{field_name} hex value must be 32 bytes"
            )));
        }

        return Ok(H256::from_slice(&bytes));
    }

    if value.len() > 32 {
        return Err(AuthError::bad_request(format!(
            "{field_name} text value must be 32 bytes or fewer"
        )));
    }

    let mut bytes = [0_u8; 32];
    bytes[..value.len()].copy_from_slice(value.as_bytes());
    Ok(H256::from(bytes))
}

pub fn format_h256(value: H256) -> String {
    format!("{value:#x}")
}

pub fn bytes32_reason(value: H256) -> String {
    bytes32_text_from_hex(&format_h256(value)).unwrap_or_else(|| format_h256(value))
}

pub fn bytes32_text_from_hex(raw: &str) -> Option<String> {
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

pub fn parse_bytes_input(raw: Option<&str>, field_name: &str) -> Result<Bytes, AuthError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Bytes::default());
    };

    if let Some(stripped) = value.strip_prefix("0x") {
        let bytes = hex::decode(stripped)
            .map_err(|_| AuthError::bad_request(format!("invalid {field_name} hex bytes")))?;
        return Ok(Bytes::from(bytes));
    }

    Ok(Bytes::from(value.as_bytes().to_vec()))
}

pub fn asset_state_label(state: u8) -> &'static str {
    match state {
        0 => "active",
        1 => "paused",
        2 => "matured",
        3 => "defaulted",
        4 => "liquidated",
        _ => "unknown",
    }
}

pub fn parse_asset_state(raw: &str) -> Result<u8, AuthError> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "0" | "active" => Ok(0),
        "1" | "paused" => Ok(1),
        "2" | "matured" => Ok(2),
        "3" | "defaulted" => Ok(3),
        "4" | "liquidated" => Ok(4),
        _ => Err(AuthError::bad_request(
            "invalid asset state, expected active|paused|matured|defaulted|liquidated",
        )),
    }
}
