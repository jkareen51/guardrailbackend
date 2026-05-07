use anyhow::Result;
use chrono::Utc;
use ethers_contract::Contract;
use ethers_core::{
    abi::AbiParser,
    types::{Address, U256},
};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::LocalWallet;
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        auth::{crud as auth_crud, error::AuthError},
        faucet::{
            crud,
            schema::{FaucetUsdcBalanceResponse, FaucetUsdcResponse},
        },
    },
    service::{
        auth::normalize_wallet_address,
        chain::{admin_signer, parse_contract_address, u256_to_string, wait_for_receipt},
        rpc,
    },
};

pub async fn request_usdc_faucet(
    state: &AppState,
    user_id: Uuid,
    requested_amount: &str,
) -> Result<FaucetUsdcResponse, AuthError> {
    let wallet = auth_crud::get_wallet_for_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::forbidden("user wallet is not linked"))?;
    let recipient = normalize_wallet_address(&wallet.wallet_address)?;
    let amount = resolve_faucet_amount(&state.env, requested_amount).await?;

    tracing::info!(
        recipient = %recipient,
        requested_amount,
        payment_token_decimals = state.env.payment_token_decimals,
        base_unit_amount = %amount,
        "faucet request parsed"
    );

    let tx_hash = send_usdc_mint_transaction(&state.env, &recipient, amount).await?;
    let record = crud::insert_faucet_request(
        &state.db,
        user_id,
        &recipient,
        &state.env.payment_token_address,
        &amount.to_string(),
        &tx_hash,
    )
    .await?;
    let balance = read_usdc_balance(state, &recipient).await?;
    Ok(FaucetUsdcResponse {
        token_address: state.env.payment_token_address.clone(),
        recipient,
        wallet_account_kind: wallet.account_kind,
        amount: record.amount,
        balance: u256_to_string(balance),
        tx_hash: record.tx_hash,
        requested_at: record.requested_at,
        next_available_at: record.requested_at,
        cooldown_seconds: 0,
    })
}

pub async fn get_mock_usdc_balance(
    state: &AppState,
    address: &str,
) -> Result<FaucetUsdcBalanceResponse, AuthError> {
    let address = normalize_wallet_address(address)?;
    let balance = read_usdc_balance(state, &address).await?;

    Ok(FaucetUsdcBalanceResponse {
        token_address: state.env.payment_token_address.clone(),
        address,
        balance: u256_to_string(balance),
        queried_at: Utc::now(),
    })
}

async fn read_usdc_balance(state: &AppState, address: &str) -> Result<U256, AuthError> {
    let contract = read_token_contract(&state.env).await.map_err(|error| {
        AuthError::internal("failed to build faucet token read contract", error)
    })?;
    let address = normalize_wallet_address(address)?;
    let account = address
        .parse::<Address>()
        .map_err(|error| AuthError::internal("invalid faucet balance address", error))?;

    contract
        .method::<_, U256>("balanceOf", account)
        .map_err(|error| AuthError::internal("failed to build balanceOf call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to query balanceOf", error))
}

async fn read_token_contract(env: &Environment) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.payment_token_address)?,
        faucet_token_abi()?,
        provider,
    ))
}

async fn write_token_contract(
    env: &Environment,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.payment_token_address)
            .map_err(|error| AuthError::internal("invalid payment token address", error))?,
        faucet_token_abi()
            .map_err(|error| AuthError::internal("failed to build faucet token ABI", error))?,
        signer,
    ))
}

async fn send_usdc_mint_transaction(
    env: &Environment,
    recipient: &str,
    amount: U256,
) -> Result<String, AuthError> {
    let contract = write_token_contract(env).await?;
    let recipient = recipient
        .parse::<Address>()
        .map_err(|error| AuthError::internal("invalid faucet recipient address", error))?;
    let call = contract
        .method::<_, ()>("mint", (recipient, amount))
        .map_err(|error| AuthError::internal("failed to build faucet mint transaction", error))?;
    let pending = call
        .send()
        .await
        .map_err(|error| AuthError::internal("failed to submit faucet mint transaction", error))?;

    wait_for_receipt(pending).await
}

fn faucet_token_abi() -> Result<ethers_core::abi::Abi> {
    AbiParser::default()
        .parse(&[
            "function mint(address to, uint256 amount)",
            "function balanceOf(address account) view returns (uint256)",
        ])
        .map_err(Into::into)
}

async fn resolve_faucet_amount(
    env: &Environment,
    requested_amount: &str,
) -> Result<U256, AuthError> {
    let raw = requested_amount.trim();
    if raw.is_empty() {
        return Err(AuthError::bad_request("amount is required"));
    }

    let decimals = env.payment_token_decimals;
    parse_display_amount(raw, decimals)
}

fn parse_display_amount(raw: &str, decimals: u8) -> Result<U256, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request("amount is required"));
    }
    if decimals > 28 {
        return Err(AuthError::internal(
            "unsupported faucet token decimals",
            format!("decimals {decimals} exceeds supported precision"),
        ));
    }

    let amount = Decimal::from_str(value)
        .map_err(|_| AuthError::bad_request("amount must be a decimal string"))?;
    if amount <= Decimal::ZERO {
        return Err(AuthError::bad_request("amount must be greater than zero"));
    }

    let scaled = (amount * decimal_power_of_ten(decimals))
        .normalize()
        .to_string();
    if scaled.contains('.') {
        return Err(AuthError::bad_request(format!(
            "amount supports up to {decimals} decimal places"
        )));
    }

    U256::from_dec_str(&scaled)
        .map_err(|_| AuthError::bad_request("amount is too large to fit into uint256"))
}

fn decimal_power_of_ten(decimals: u8) -> Decimal {
    let mut value = Decimal::ONE;
    for _ in 0..decimals {
        value *= Decimal::TEN;
    }
    value
}

#[cfg(test)]
mod tests {
    use ethers_core::types::U256;

    use super::parse_display_amount;

    #[test]
    fn parses_whole_display_amount_using_payment_token_decimals() {
        let amount = parse_display_amount("10", 6).expect("amount should parse");
        assert_eq!(amount, U256::from(10_000_000u64));
    }

    #[test]
    fn parses_fractional_display_amount_using_payment_token_decimals() {
        let amount = parse_display_amount("100.25", 6).expect("amount should parse");
        assert_eq!(amount, U256::from(100_250_000u64));
    }
}
