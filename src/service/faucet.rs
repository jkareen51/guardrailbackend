use anyhow::{Context, Result};
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
    requested_amount: Option<&str>,
) -> Result<FaucetUsdcResponse, AuthError> {
    let wallet = auth_crud::get_wallet_for_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::forbidden("user wallet is not linked"))?;
    let recipient = normalize_wallet_address(&wallet.wallet_address)?;
    let amount = resolve_faucet_amount(state, requested_amount).await?;

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
            "function decimals() view returns (uint8)",
        ])
        .map_err(Into::into)
}

async fn resolve_faucet_amount(
    state: &AppState,
    requested_amount: Option<&str>,
) -> Result<U256, AuthError> {
    match requested_amount.map(str::trim).filter(|value| !value.is_empty()) {
        Some(raw) => {
            let decimals = read_token_decimals(&state.env).await?;
            parse_display_amount(raw, decimals)
        }
        None => parse_base_unit_amount(&state.env.faucet_usdc_amount).map_err(|error| {
            AuthError::internal("invalid FAUCET_USDC_AMOUNT configuration", error)
        }),
    }
}

async fn read_token_decimals(env: &Environment) -> Result<u8, AuthError> {
    let contract = read_token_contract(env)
        .await
        .map_err(|error| AuthError::internal("failed to build faucet token read contract", error))?;

    contract
        .method::<_, u8>("decimals", ())
        .map_err(|error| AuthError::internal("failed to build decimals call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to query decimals", error))
}

fn parse_base_unit_amount(raw: &str) -> Result<U256> {
    let value = raw.trim();
    if value.is_empty() {
        anyhow::bail!("amount is required");
    }

    let amount =
        U256::from_dec_str(value).with_context(|| "amount must be a base-10 integer string")?;
    if amount.is_zero() {
        anyhow::bail!("amount must be greater than zero");
    }

    Ok(amount)
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

    let scaled = (amount * decimal_power_of_ten(decimals)).normalize().to_string();
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
