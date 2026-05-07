use anyhow::Result;
use ethers_contract::Contract;
use ethers_core::{
    abi::{Detokenize, Tokenize},
    types::{Address, U256},
};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::LocalWallet;
use uuid::Uuid;

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        auth::error::AuthError,
        treasury::{
            crud,
            model::{TreasuryAssetRecord, TreasuryStatusRecord},
            schema::{
                AdminApproveTreasuryPaymentTokenRequest, AdminDepositAssetLiquidityRequest,
                AdminDepositYieldRequest, AdminEmergencyWithdrawRequest,
                AdminReleaseCapitalRequest, TreasuryAssetResponse, TreasuryAssetWriteResponse,
                TreasuryPaymentTokenApprovalResponse, TreasuryStatusResponse,
                TreasuryStatusWriteResponse,
            },
        },
    },
    service::{
        asset::abi::erc20_abi,
        chain::{
            admin_signer, format_address, parse_address, parse_bytes_input, parse_bytes32_input,
            parse_contract_address, parse_u256, wait_for_receipt,
        },
        rpc,
    },
};

use self::abi::treasury_abi;

pub mod abi;

pub async fn get_treasury_status(state: &AppState) -> Result<TreasuryStatusResponse, AuthError> {
    match crud::get_treasury_status(&state.db, &state.env.treasury_address).await? {
        Some(record) => Ok(TreasuryStatusResponse::from(record)),
        None => Ok(TreasuryStatusResponse::from(
            sync_treasury_status(state, None, None).await?,
        )),
    }
}

pub async fn get_treasury_asset(
    state: &AppState,
    asset_address: &str,
) -> Result<TreasuryAssetResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let asset_address_string = format_address(asset_address);

    match crud::get_treasury_asset(&state.db, &asset_address_string).await? {
        Some(record) => Ok(TreasuryAssetResponse::from(record)),
        None => Ok(TreasuryAssetResponse::from(
            sync_treasury_asset(state, asset_address, None, None).await?,
        )),
    }
}

pub async fn approve_payment_token(
    state: &AppState,
    _actor_user_id: Uuid,
    payload: AdminApproveTreasuryPaymentTokenRequest,
) -> Result<TreasuryPaymentTokenApprovalResponse, AuthError> {
    let amount = parse_u256(&payload.amount, "amount")?;
    let tx_hash = send_erc20_transaction::<_, bool>(
        &state.env,
        parse_address(&state.env.payment_token_address)?,
        "approve",
        (parse_address(&state.env.treasury_address)?, amount),
        "failed to submit payment token approve transaction",
    )
    .await?;

    Ok(TreasuryPaymentTokenApprovalResponse {
        tx_hash,
        payment_token_address: state.env.payment_token_address.clone(),
        treasury_address: state.env.treasury_address.clone(),
        approved_amount: amount.to_string(),
    })
}

pub async fn deposit_asset_liquidity(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminDepositAssetLiquidityRequest,
) -> Result<TreasuryAssetWriteResponse, AuthError> {
    let asset_address = parse_address(&payload.asset_address)?;
    let amount = parse_u256(&payload.amount, "amount")?;
    ensure_operator_liquidity_capacity(&state.env, amount).await?;
    let tx_hash = send_treasury_transaction::<_, ()>(
        &state.env,
        "depositAssetLiquidity",
        (asset_address, amount),
        "failed to submit depositAssetLiquidity transaction",
    )
    .await?;

    treasury_asset_write_response(state, actor_user_id, asset_address, &tx_hash).await
}

pub async fn release_capital(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminReleaseCapitalRequest,
) -> Result<TreasuryAssetWriteResponse, AuthError> {
    let asset_address = parse_address(&payload.asset_address)?;
    let amount = parse_u256(&payload.amount, "amount")?;
    let recipient_wallet = parse_address(&payload.recipient_wallet)?;
    let reference_id = parse_bytes32_input(&payload.reference_id, "reference_id")?;
    let tx_hash = send_treasury_transaction::<_, ()>(
        &state.env,
        "releaseCapital",
        (asset_address, amount, recipient_wallet, reference_id),
        "failed to submit releaseCapital transaction",
    )
    .await?;

    treasury_asset_write_response(state, actor_user_id, asset_address, &tx_hash).await
}

pub async fn deposit_yield(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminDepositYieldRequest,
) -> Result<TreasuryAssetWriteResponse, AuthError> {
    let asset_address = parse_address(&payload.asset_address)?;
    let amount = parse_u256(&payload.amount, "amount")?;
    let data = parse_bytes_input(payload.data.as_deref(), "data")?;
    let tx_hash = send_treasury_transaction::<_, ()>(
        &state.env,
        "depositYield",
        (asset_address, amount, data),
        "failed to submit depositYield transaction",
    )
    .await?;

    treasury_asset_write_response(state, actor_user_id, asset_address, &tx_hash).await
}

pub async fn emergency_withdraw(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminEmergencyWithdrawRequest,
) -> Result<TreasuryStatusWriteResponse, AuthError> {
    let token_address = parse_address(&payload.token_address)?;
    let amount = parse_u256(&payload.amount, "amount")?;
    let recipient_wallet = parse_address(&payload.recipient_wallet)?;
    let tx_hash = send_treasury_transaction::<_, ()>(
        &state.env,
        "emergencyWithdraw",
        (token_address, amount, recipient_wallet),
        "failed to submit emergencyWithdraw transaction",
    )
    .await?;

    let treasury = sync_treasury_status(state, Some(actor_user_id), Some(&tx_hash)).await?;
    Ok(TreasuryStatusWriteResponse {
        tx_hash,
        treasury: TreasuryStatusResponse::from(treasury),
    })
}

pub async fn pause_treasury(
    state: &AppState,
    actor_user_id: Uuid,
) -> Result<TreasuryStatusWriteResponse, AuthError> {
    let tx_hash = send_treasury_transaction::<_, ()>(
        &state.env,
        "pause",
        (),
        "failed to submit treasury pause transaction",
    )
    .await?;

    let treasury = sync_treasury_status(state, Some(actor_user_id), Some(&tx_hash)).await?;
    Ok(TreasuryStatusWriteResponse {
        tx_hash,
        treasury: TreasuryStatusResponse::from(treasury),
    })
}

pub async fn unpause_treasury(
    state: &AppState,
    actor_user_id: Uuid,
) -> Result<TreasuryStatusWriteResponse, AuthError> {
    let tx_hash = send_treasury_transaction::<_, ()>(
        &state.env,
        "unpause",
        (),
        "failed to submit treasury unpause transaction",
    )
    .await?;

    let treasury = sync_treasury_status(state, Some(actor_user_id), Some(&tx_hash)).await?;
    Ok(TreasuryStatusWriteResponse {
        tx_hash,
        treasury: TreasuryStatusResponse::from(treasury),
    })
}

pub async fn register_asset_token(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
) -> Result<TreasuryAssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let tx_hash = send_treasury_transaction::<_, ()>(
        &state.env,
        "registerAssetToken",
        asset_address,
        "failed to submit registerAssetToken transaction",
    )
    .await?;

    treasury_asset_write_response(state, actor_user_id, asset_address, &tx_hash).await
}

async fn treasury_asset_write_response(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: Address,
    tx_hash: &str,
) -> Result<TreasuryAssetWriteResponse, AuthError> {
    let treasury = sync_treasury_status(state, Some(actor_user_id), Some(tx_hash)).await?;
    let asset =
        sync_treasury_asset(state, asset_address, Some(actor_user_id), Some(tx_hash)).await?;

    Ok(TreasuryAssetWriteResponse {
        tx_hash: tx_hash.to_owned(),
        treasury: TreasuryStatusResponse::from(treasury),
        asset: TreasuryAssetResponse::from(asset),
    })
}

async fn sync_treasury_status(
    state: &AppState,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<TreasuryStatusRecord, AuthError> {
    let contract = read_treasury_contract(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build treasury read contract", error))?;

    let payment_token_address = contract
        .method::<_, Address>("paymentToken", ())
        .map_err(|error| AuthError::internal("failed to build paymentToken call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call paymentToken", error))?;
    let access_control_address = contract
        .method::<_, Address>("accessControl", ())
        .map_err(|error| AuthError::internal("failed to build accessControl call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call accessControl", error))?;
    let paused = contract
        .method::<_, bool>("paused", ())
        .map_err(|error| AuthError::internal("failed to build paused call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call paused", error))?;
    let total_tracked_balance = contract
        .method::<_, U256>("totalTrackedBalance", ())
        .map_err(|error| AuthError::internal("failed to build totalTrackedBalance call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call totalTrackedBalance", error))?;
    let total_reserved_yield = contract
        .method::<_, U256>("totalReservedYield", ())
        .map_err(|error| AuthError::internal("failed to build totalReservedYield call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call totalReservedYield", error))?;
    let total_reserved_redemptions = contract
        .method::<_, U256>("totalReservedRedemptions", ())
        .map_err(|error| {
            AuthError::internal("failed to build totalReservedRedemptions call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call totalReservedRedemptions", error))?;

    crud::upsert_treasury_status(
        &state.db,
        &state.env.treasury_address,
        &format_address(payment_token_address),
        &format_address(access_control_address),
        paused,
        &total_tracked_balance.to_string(),
        &total_reserved_yield.to_string(),
        &total_reserved_redemptions.to_string(),
        updated_by_user_id,
        last_tx_hash,
    )
    .await
}

async fn sync_treasury_asset(
    state: &AppState,
    asset_address: Address,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<TreasuryAssetRecord, AuthError> {
    let contract = read_treasury_contract(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build treasury read contract", error))?;

    let balance = contract
        .method::<_, U256>("getBalance", asset_address)
        .map_err(|error| AuthError::internal("failed to build getBalance call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getBalance", error))?;
    let reserved_yield = contract
        .method::<_, U256>("getReservedYield", asset_address)
        .map_err(|error| AuthError::internal("failed to build getReservedYield call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getReservedYield", error))?;
    let reserved_redemptions = contract
        .method::<_, U256>("getReservedRedemptions", asset_address)
        .map_err(|error| AuthError::internal("failed to build getReservedRedemptions call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getReservedRedemptions", error))?;
    let available_liquidity = contract
        .method::<_, U256>("getAvailableLiquidity", asset_address)
        .map_err(|error| AuthError::internal("failed to build getAvailableLiquidity call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getAvailableLiquidity", error))?;
    let registered_asset_token = contract
        .method::<_, bool>("registeredAssetTokens", asset_address)
        .map_err(|error| AuthError::internal("failed to build registeredAssetTokens call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call registeredAssetTokens", error))?;

    crud::upsert_treasury_asset(
        &state.db,
        &format_address(asset_address),
        &balance.to_string(),
        &reserved_yield.to_string(),
        &reserved_redemptions.to_string(),
        &available_liquidity.to_string(),
        registered_asset_token,
        updated_by_user_id,
        last_tx_hash,
    )
    .await
}

async fn read_treasury_contract(env: &Environment) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.treasury_address)?,
        treasury_abi()?,
        provider,
    ))
}

async fn read_erc20_contract(
    env: &Environment,
    token_address: Address,
) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(token_address, erc20_abi()?, provider))
}

async fn write_treasury_contract(
    env: &Environment,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.treasury_address)
            .map_err(|error| AuthError::internal("invalid treasury address", error))?,
        treasury_abi()
            .map_err(|error| AuthError::internal("failed to build treasury ABI", error))?,
        signer,
    ))
}

async fn write_erc20_contract(
    env: &Environment,
    token_address: Address,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        token_address,
        erc20_abi().map_err(|error| AuthError::internal("failed to build ERC20 ABI", error))?,
        signer,
    ))
}

async fn send_treasury_transaction<T, D>(
    env: &Environment,
    method: &str,
    args: T,
    error_context: &'static str,
) -> Result<String, AuthError>
where
    T: Tokenize,
    D: Detokenize,
{
    let contract = write_treasury_contract(env).await?;
    let call = contract
        .method::<_, D>(method, args)
        .map_err(|error| AuthError::internal("failed to build treasury transaction", error))?;
    let pending = call
        .send()
        .await
        .map_err(|error| AuthError::internal(error_context, error))?;
    wait_for_receipt(pending).await
}

async fn send_erc20_transaction<T, D>(
    env: &Environment,
    token_address: Address,
    method: &str,
    args: T,
    error_context: &'static str,
) -> Result<String, AuthError>
where
    T: Tokenize,
    D: Detokenize,
{
    let contract = write_erc20_contract(env, token_address).await?;
    let call = contract
        .method::<_, D>(method, args)
        .map_err(|error| AuthError::internal("failed to build ERC20 transaction", error))?;
    let pending = call
        .send()
        .await
        .map_err(|error| AuthError::internal(error_context, error))?;
    wait_for_receipt(pending).await
}

async fn ensure_operator_liquidity_capacity(
    env: &Environment,
    amount: U256,
) -> Result<(), AuthError> {
    let signer = admin_signer(env).await?;
    let operator_address = signer.address();
    let treasury_address = parse_address(&env.treasury_address)?;
    let payment_token_address = parse_address(&env.payment_token_address)?;
    let token_contract = read_erc20_contract(env, payment_token_address)
        .await
        .map_err(|error| {
            AuthError::internal("failed to build payment token read contract", error)
        })?;

    let operator_balance = token_contract
        .method::<_, U256>("balanceOf", operator_address)
        .map_err(|error| {
            AuthError::internal("failed to build payment token balanceOf call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call payment token balanceOf", error))?;
    if operator_balance < amount {
        return Err(AuthError::bad_request(format!(
            "insufficient operator payment-token balance: required={}, available={}",
            amount, operator_balance
        )));
    }

    let operator_allowance = token_contract
        .method::<_, U256>("allowance", (operator_address, treasury_address))
        .map_err(|error| {
            AuthError::internal("failed to build payment token allowance call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call payment token allowance", error))?;
    if operator_allowance < amount {
        return Err(AuthError::bad_request(format!(
            "insufficient treasury allowance from operator wallet: required={}, approved={}",
            amount, operator_allowance
        )));
    }

    Ok(())
}
