use anyhow::Result;
use ethers_contract::Contract;
use ethers_core::{
    abi::{Detokenize, Tokenize},
    types::{Address, H256, U256},
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
        oracle::{
            crud,
            model::{OracleDocumentRecord, OracleTrustedOracleRecord, OracleValuationRecord},
            schema::{
                AdminAnchorDocumentRequest, AdminSetTrustedOracleRequest,
                AdminSubmitValuationAndSyncPricingRequest, AdminSubmitValuationRequest,
                OracleDocumentResponse, OracleDocumentWriteResponse, OracleTrustedOracleResponse,
                OracleTrustedOracleWriteResponse, OracleValuationResponse,
                OracleValuationWriteResponse,
            },
        },
    },
    service::{
        chain::{
            admin_signer, format_address, format_h256, parse_address, parse_bytes32_input,
            parse_contract_address, parse_u256, wait_for_receipt,
        },
        rpc,
    },
};

use self::abi::oracle_bridge_abi;

pub mod abi;

type AssetValuationTuple = (U256, U256, u64, H256);

pub async fn get_trusted_oracle(
    state: &AppState,
    oracle_address: &str,
) -> Result<OracleTrustedOracleResponse, AuthError> {
    let oracle_address = parse_address(oracle_address)?;
    let oracle_address_string = format_address(oracle_address);

    match sync_trusted_oracle(state, oracle_address, None, None).await {
        Ok(record) => Ok(OracleTrustedOracleResponse::from(record)),
        Err(error) => match crud::get_trusted_oracle(&state.db, &oracle_address_string).await? {
            Some(record) => Ok(OracleTrustedOracleResponse::from(record)),
            None => Err(error),
        },
    }
}

pub async fn set_trusted_oracle(
    state: &AppState,
    actor_user_id: Uuid,
    oracle_address: &str,
    payload: AdminSetTrustedOracleRequest,
) -> Result<OracleTrustedOracleWriteResponse, AuthError> {
    let oracle_address = parse_address(oracle_address)?;
    let tx_hash = send_oracle_transaction::<_, ()>(
        &state.env,
        "setTrustedOracle",
        (oracle_address, payload.trusted),
        "failed to submit setTrustedOracle transaction",
    )
    .await?;

    let trusted_oracle = crud::upsert_trusted_oracle(
        &state.db,
        &format_address(oracle_address),
        payload.trusted,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(OracleTrustedOracleWriteResponse {
        tx_hash,
        trusted_oracle: OracleTrustedOracleResponse::from(trusted_oracle),
    })
}

pub async fn get_valuation(
    state: &AppState,
    asset_address: &str,
) -> Result<OracleValuationResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let asset_address_string = format_address(asset_address);

    match sync_valuation(state, asset_address, None, None).await {
        Ok(record) => Ok(OracleValuationResponse::from(record)),
        Err(error) => match crud::get_valuation(&state.db, &asset_address_string).await? {
            Some(record) => Ok(OracleValuationResponse::from(record)),
            None => Err(error),
        },
    }
}

pub async fn submit_valuation(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminSubmitValuationRequest,
) -> Result<OracleValuationWriteResponse, AuthError> {
    let asset_address = parse_address(&payload.asset_address)?;
    let asset_value = parse_u256(&payload.asset_value, "asset_value")?;
    let nav_per_token = parse_u256(&payload.nav_per_token, "nav_per_token")?;
    let reference_id = parse_bytes32_input(&payload.reference_id, "reference_id")?;
    let tx_hash = send_oracle_transaction::<_, ()>(
        &state.env,
        "submitValuation",
        (asset_address, asset_value, nav_per_token, reference_id),
        "failed to submit submitValuation transaction",
    )
    .await?;

    let valuation =
        sync_valuation(state, asset_address, Some(actor_user_id), Some(&tx_hash)).await?;

    Ok(OracleValuationWriteResponse {
        tx_hash,
        valuation: OracleValuationResponse::from(valuation),
    })
}

pub async fn submit_valuation_and_sync_pricing(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminSubmitValuationAndSyncPricingRequest,
) -> Result<OracleValuationWriteResponse, AuthError> {
    let asset_address = parse_address(&payload.asset_address)?;
    let asset_value = parse_u256(&payload.asset_value, "asset_value")?;
    let nav_per_token = parse_u256(&payload.nav_per_token, "nav_per_token")?;
    let subscription_price = parse_u256(&payload.subscription_price, "subscription_price")?;
    let redemption_price = parse_u256(&payload.redemption_price, "redemption_price")?;
    let reference_id = parse_bytes32_input(&payload.reference_id, "reference_id")?;
    let tx_hash = send_oracle_transaction::<_, ()>(
        &state.env,
        "submitValuationAndSyncPricing",
        (
            asset_address,
            asset_value,
            nav_per_token,
            subscription_price,
            redemption_price,
            reference_id,
        ),
        "failed to submit submitValuationAndSyncPricing transaction",
    )
    .await?;

    let valuation =
        sync_valuation(state, asset_address, Some(actor_user_id), Some(&tx_hash)).await?;

    Ok(OracleValuationWriteResponse {
        tx_hash,
        valuation: OracleValuationResponse::from(valuation),
    })
}

pub async fn get_document(
    state: &AppState,
    asset_address: &str,
    document_type: &str,
) -> Result<OracleDocumentResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let document_type = parse_bytes32_input(document_type, "document_type")?;
    let asset_address_string = format_address(asset_address);
    let document_type_string = format_h256(document_type);

    match sync_document(state, asset_address, document_type, None, None).await {
        Ok(record) => Ok(OracleDocumentResponse::from(record)),
        Err(error) => {
            match crud::get_document(&state.db, &asset_address_string, &document_type_string)
                .await?
            {
                Some(record) => Ok(OracleDocumentResponse::from(record)),
                None => Err(error),
            }
        }
    }
}

pub async fn anchor_document(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    document_type: &str,
    payload: AdminAnchorDocumentRequest,
) -> Result<OracleDocumentWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let document_type = parse_bytes32_input(document_type, "document_type")?;
    let document_hash = parse_bytes32_input(&payload.document_hash, "document_hash")?;
    let reference_id = parse_bytes32_input(&payload.reference_id, "reference_id")?;
    let tx_hash = send_oracle_transaction::<_, ()>(
        &state.env,
        "anchorDocument",
        (asset_address, document_type, document_hash, reference_id),
        "failed to submit anchorDocument transaction",
    )
    .await?;

    let document = sync_document(
        state,
        asset_address,
        document_type,
        Some((document_hash, reference_id)),
        Some((actor_user_id, &tx_hash)),
    )
    .await?;

    Ok(OracleDocumentWriteResponse {
        tx_hash,
        document: OracleDocumentResponse::from(document),
    })
}

async fn sync_trusted_oracle(
    state: &AppState,
    oracle_address: Address,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<OracleTrustedOracleRecord, AuthError> {
    let contract = read_oracle_contract(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build oracle read contract", error))?;
    let is_trusted = contract
        .method::<_, bool>("trustedOracles", oracle_address)
        .map_err(|error| AuthError::internal("failed to build trustedOracles call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call trustedOracles", error))?;

    crud::upsert_trusted_oracle(
        &state.db,
        &format_address(oracle_address),
        is_trusted,
        updated_by_user_id,
        last_tx_hash,
    )
    .await
}

async fn sync_valuation(
    state: &AppState,
    asset_address: Address,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<OracleValuationRecord, AuthError> {
    let contract = read_oracle_contract(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build oracle read contract", error))?;
    let (asset_value, nav_per_token, updated_at, reference_id) = contract
        .method::<_, AssetValuationTuple>("getLatestValuation", asset_address)
        .map_err(|error| AuthError::internal("failed to build getLatestValuation call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getLatestValuation", error))?;

    crud::upsert_valuation(
        &state.db,
        &format_address(asset_address),
        &asset_value.to_string(),
        &nav_per_token.to_string(),
        i64::try_from(updated_at)
            .map_err(|_| AuthError::bad_request("valuation updated_at is out of range"))?,
        &format_h256(reference_id),
        updated_by_user_id,
        last_tx_hash,
    )
    .await
}

async fn sync_document(
    state: &AppState,
    asset_address: Address,
    document_type: H256,
    write_values: Option<(H256, H256)>,
    write_meta: Option<(Uuid, &str)>,
) -> Result<OracleDocumentRecord, AuthError> {
    let contract = read_oracle_contract(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build oracle read contract", error))?;
    let document_hash = contract
        .method::<_, H256>("getDocumentHash", (asset_address, document_type))
        .map_err(|error| AuthError::internal("failed to build getDocumentHash call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getDocumentHash", error))?;
    let (updated_by_user_id, last_tx_hash) = match write_meta {
        Some((user_id, tx_hash)) => (Some(user_id), Some(tx_hash)),
        None => (None, None),
    };
    let reference_id = write_values
        .map(|(_, reference_id)| reference_id)
        .unwrap_or_default();

    crud::upsert_document(
        &state.db,
        &format_address(asset_address),
        &format_h256(document_type),
        &format_h256(document_hash),
        &format_h256(reference_id),
        updated_by_user_id,
        last_tx_hash,
    )
    .await
}

async fn read_oracle_contract(env: &Environment) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.oracle_data_bridge_address)?,
        oracle_bridge_abi()?,
        provider,
    ))
}

async fn write_oracle_contract(
    env: &Environment,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.oracle_data_bridge_address)
            .map_err(|error| AuthError::internal("invalid oracle data bridge address", error))?,
        oracle_bridge_abi()
            .map_err(|error| AuthError::internal("failed to build oracle bridge ABI", error))?,
        signer,
    ))
}

async fn send_oracle_transaction<T, D>(
    env: &Environment,
    method: &str,
    args: T,
    error_context: &'static str,
) -> Result<String, AuthError>
where
    T: Tokenize,
    D: Detokenize,
{
    let contract = write_oracle_contract(env).await?;
    let call = contract
        .method::<_, D>(method, args)
        .map_err(|error| AuthError::internal("failed to build oracle transaction", error))?;
    let pending = call
        .send()
        .await
        .map_err(|error| AuthError::internal(error_context, error))?;
    wait_for_receipt(pending).await
}
