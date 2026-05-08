use std::{future::Future, str::FromStr};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, TimeZone, Utc};
use ethers_contract::Contract;
use ethers_core::{
    abi::{AbiParser, Detokenize, Token, Tokenize, encode},
    types::{Address, Bytes, H256, U256},
    utils::keccak256,
};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::LocalWallet;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        asset::{
            crud,
            model::{AssetPriceHistoryRecord, AssetRecord, AssetTypeRecord},
            schema::{
                AdminBurnAssetRequest, AdminControllerTransferRequest, AdminCreateAssetRequest,
                AdminIssueAssetRequest, AdminProcessRedemptionRequest,
                AdminRegisterAssetTypeRequest, AdminSetAssetCatalogRequest,
                AdminSetAssetComplianceRegistryRequest, AdminSetAssetMetadataRequest,
                AdminSetAssetPriceRequest, AdminSetAssetPricingRequest,
                AdminSetAssetSelfServicePurchaseRequest, AdminSetAssetStateRequest,
                AdminSetAssetTreasuryRequest, AdminSetFactoryComplianceRegistryRequest,
                AdminSetFactoryTreasuryRequest, AssetArchiveWriteResponse,
                AssetCatalogWriteResponse, AssetDetailQuery, AssetDetailResponse,
                AssetFactoryStatusResponse, AssetFactoryWriteResponse, AssetHistoryCandleResponse,
                AssetHistoryQuery, AssetHistoryResponse, AssetHolderStateResponse,
                AssetListResponse, AssetPendingRedemptionsResponse, AssetPreviewRequest,
                AssetPreviewResponse, AssetResponse, AssetTransferCheckResponse,
                AssetTypeListResponse, AssetTypeResponse, AssetTypeWriteResponse,
                AssetWriteResponse, GaslessApprovePaymentTokenRequest, GaslessAssetActionResponse,
                GaslessCancelRedemptionRequest, GaslessClaimYieldRequest,
                GaslessPurchaseAssetRequest, GaslessRedeemAssetRequest, ListAssetsQuery,
                PendingRedemptionItem,
            },
        },
        auth::{crud as auth_crud, error::AuthError},
        compliance::schema::ComplianceCheckSubscribeRequest,
        oracle::{crud as oracle_crud, model::OracleValuationHistoryRecord},
    },
    service::{
        chain::{
            admin_signer, asset_state_label, bytes32_reason, format_address, format_h256,
            parse_address, parse_asset_state, parse_bytes_input, parse_bytes32_input,
            parse_contract_address, parse_u256, wait_for_receipt,
        },
        compliance, gasless, oracle, rpc, treasury,
    },
};

use self::abi::{asset_factory_abi, base_asset_token_abi, erc20_abi};

pub mod abi;

const DEFAULT_LIST_LIMIT: i64 = 20;
const MAX_LIST_LIMIT: i64 = 100;
const DEFAULT_HISTORY_RANGE: &str = "1day";

#[derive(Debug, Clone)]
struct AssetFactorySnapshot {
    access_control_address: String,
    compliance_registry_address: String,
    treasury_address: String,
    paused: bool,
    total_assets_created: String,
}

#[derive(Debug, Clone)]
struct AssetSnapshot {
    asset_address: String,
    proposal_id: String,
    asset_type_id: String,
    name: String,
    symbol: String,
    max_supply: String,
    total_supply: String,
    asset_state: i32,
    asset_state_label: String,
    controllable: bool,
    self_service_purchase_enabled: bool,
    price_per_token: String,
    redemption_price_per_token: String,
    treasury_address: String,
    compliance_registry_address: String,
    payment_token_address: String,
    metadata_hash: String,
    holder_count: String,
    total_pending_redemptions: String,
}

#[derive(Debug, Clone)]
struct AssetHolderSnapshot {
    wallet_address: String,
    balance: String,
    claimable_yield: String,
    accumulative_yield: String,
    pending_redemption: String,
    locked_balance: String,
    unlocked_balance: String,
    payment_token_balance: String,
    payment_token_allowance_to_treasury: String,
}

#[derive(Debug, Clone, Copy)]
struct AssetHistoryWindow {
    range: &'static str,
    interval_label: &'static str,
    interval: Duration,
    observed_from: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct HistorySample {
    observed_at: DateTime<Utc>,
    value: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct WalletAccountLookup {
    user_id: Uuid,
    wallet_address: String,
    email: Option<String>,
    display_name: Option<String>,
}

struct NormalizedAssetListQuery {
    asset_type_id: Option<String>,
    q: Option<String>,
    asset_state: Option<i32>,
    self_service_purchase_enabled: Option<bool>,
    featured: Option<bool>,
    limit: i64,
    offset: i64,
}

pub async fn get_factory_status(state: &AppState) -> Result<AssetFactoryStatusResponse, AuthError> {
    let snapshot = read_factory_status_from_chain(&state.env).await?;
    Ok(factory_status_response(&state.env, snapshot))
}

pub async fn list_asset_types(state: &AppState) -> Result<AssetTypeListResponse, AuthError> {
    match read_registered_asset_type_ids(&state.env).await {
        Ok(asset_type_ids) => {
            let mut asset_types = Vec::with_capacity(asset_type_ids.len());
            for asset_type_id in asset_type_ids {
                let record = sync_asset_type(state, asset_type_id, None, None).await?;
                asset_types.push(AssetTypeResponse::from(record));
            }
            Ok(AssetTypeListResponse { asset_types })
        }
        Err(_error) => {
            let asset_types = crud::list_asset_types(&state.db).await?;
            Ok(AssetTypeListResponse {
                asset_types: asset_types
                    .into_iter()
                    .filter(|record| record.is_registered)
                    .map(AssetTypeResponse::from)
                    .collect(),
            })
        }
    }
}

pub async fn get_asset_type(
    state: &AppState,
    asset_type_id: &str,
) -> Result<AssetTypeResponse, AuthError> {
    let asset_type_id = parse_bytes32_input(asset_type_id, "asset_type_id")?;
    let asset_type_id_hex = format_h256(asset_type_id);

    match crud::get_asset_type(&state.db, &asset_type_id_hex).await? {
        Some(record) => Ok(AssetTypeResponse::from(record)),
        None => {
            let record = sync_asset_type(state, asset_type_id, None, None).await?;
            Ok(AssetTypeResponse::from(record))
        }
    }
}

pub async fn register_asset_type(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminRegisterAssetTypeRequest,
) -> Result<AssetTypeWriteResponse, AuthError> {
    ensure_factory_signer_has_any_role(
        &state.env,
        &[("ADMIN_ROLE", role_hash("ADMIN_ROLE"))],
        "register asset types",
    )
    .await?;
    let asset_type_id = parse_bytes32_input(&payload.asset_type_id, "asset_type_id")?;
    let implementation_address = parse_address(&payload.implementation_address)?;
    let asset_type_id_hex = format_h256(asset_type_id);
    let (registered_name, registered_implementation, is_registered) =
        read_asset_type_from_chain(&state.env, asset_type_id).await?;

    if is_registered {
        crud::upsert_asset_type(
            &state.db,
            &asset_type_id_hex,
            &registered_name,
            &format_address(registered_implementation),
            true,
            Some(actor_user_id),
            None,
        )
        .await?;

        return Err(AuthError::conflict(format!(
            "asset type `{}` is already registered with implementation {}",
            asset_type_id_hex,
            format_address(registered_implementation)
        )));
    }

    let tx_hash = send_factory_transaction::<_, ()>(
        &state.env,
        "registerAssetType",
        (
            asset_type_id,
            payload.asset_type_name.clone(),
            implementation_address,
        ),
        "failed to submit registerAssetType transaction",
    )
    .await?;

    let asset_type =
        match sync_asset_type(state, asset_type_id, Some(actor_user_id), Some(&tx_hash)).await {
            Ok(record) => record,
            Err(_) => {
                crud::upsert_asset_type(
                    &state.db,
                    &format_h256(asset_type_id),
                    &payload.asset_type_name,
                    &format_address(implementation_address),
                    true,
                    Some(actor_user_id),
                    Some(&tx_hash),
                )
                .await?
            }
        };

    Ok(AssetTypeWriteResponse {
        tx_hash,
        asset_type: AssetTypeResponse::from(asset_type),
    })
}

pub async fn unregister_asset_type(
    state: &AppState,
    actor_user_id: Uuid,
    asset_type_id: &str,
) -> Result<AssetTypeWriteResponse, AuthError> {
    ensure_factory_signer_has_any_role(
        &state.env,
        &[("ADMIN_ROLE", role_hash("ADMIN_ROLE"))],
        "unregister asset types",
    )
    .await?;
    let asset_type_id = parse_bytes32_input(asset_type_id, "asset_type_id")?;

    let tx_hash = send_factory_transaction::<_, ()>(
        &state.env,
        "unregisterAssetType",
        asset_type_id,
        "failed to submit unregisterAssetType transaction",
    )
    .await?;

    let record = sync_asset_type(state, asset_type_id, Some(actor_user_id), Some(&tx_hash)).await?;

    Ok(AssetTypeWriteResponse {
        tx_hash,
        asset_type: AssetTypeResponse::from(record),
    })
}

pub async fn pause_factory(
    state: &AppState,
    _actor_user_id: Uuid,
) -> Result<AssetFactoryWriteResponse, AuthError> {
    ensure_factory_signer_has_any_role(
        &state.env,
        &[("ADMIN_ROLE", role_hash("ADMIN_ROLE"))],
        "pause the asset factory",
    )
    .await?;
    let tx_hash = send_factory_transaction::<_, ()>(
        &state.env,
        "pauseFactory",
        (),
        "failed to submit pauseFactory transaction",
    )
    .await?;

    let factory = get_factory_status(state).await?;
    Ok(AssetFactoryWriteResponse { tx_hash, factory })
}

pub async fn unpause_factory(
    state: &AppState,
    _actor_user_id: Uuid,
) -> Result<AssetFactoryWriteResponse, AuthError> {
    ensure_factory_signer_has_any_role(
        &state.env,
        &[("ADMIN_ROLE", role_hash("ADMIN_ROLE"))],
        "unpause the asset factory",
    )
    .await?;
    let tx_hash = send_factory_transaction::<_, ()>(
        &state.env,
        "unpauseFactory",
        (),
        "failed to submit unpauseFactory transaction",
    )
    .await?;

    let factory = get_factory_status(state).await?;
    Ok(AssetFactoryWriteResponse { tx_hash, factory })
}

pub async fn set_factory_compliance_registry(
    state: &AppState,
    _actor_user_id: Uuid,
    payload: AdminSetFactoryComplianceRegistryRequest,
) -> Result<AssetFactoryWriteResponse, AuthError> {
    ensure_factory_signer_has_any_role(
        &state.env,
        &[("ADMIN_ROLE", role_hash("ADMIN_ROLE"))],
        "update the asset factory compliance registry",
    )
    .await?;
    let compliance_registry_address = parse_address(&payload.compliance_registry_address)?;

    let tx_hash = send_factory_transaction::<_, ()>(
        &state.env,
        "setComplianceRegistry",
        compliance_registry_address,
        "failed to submit setComplianceRegistry factory transaction",
    )
    .await?;

    let factory = get_factory_status(state).await?;
    Ok(AssetFactoryWriteResponse { tx_hash, factory })
}

pub async fn set_factory_treasury(
    state: &AppState,
    _actor_user_id: Uuid,
    payload: AdminSetFactoryTreasuryRequest,
) -> Result<AssetFactoryWriteResponse, AuthError> {
    ensure_factory_signer_has_any_role(
        &state.env,
        &[("ADMIN_ROLE", role_hash("ADMIN_ROLE"))],
        "update the asset factory treasury",
    )
    .await?;
    let treasury_address = parse_address(&payload.treasury_address)?;

    let tx_hash = send_factory_transaction::<_, ()>(
        &state.env,
        "setTreasury",
        treasury_address,
        "failed to submit setTreasury factory transaction",
    )
    .await?;

    let factory = get_factory_status(state).await?;
    Ok(AssetFactoryWriteResponse { tx_hash, factory })
}

pub async fn create_asset(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminCreateAssetRequest,
) -> Result<AssetWriteResponse, AuthError> {
    ensure_factory_signer_has_any_role(
        &state.env,
        &[
            ("ISSUER_ROLE", role_hash("ISSUER_ROLE")),
            ("ADMIN_ROLE", role_hash("ADMIN_ROLE")),
        ],
        "create assets",
    )
    .await?;
    let proposal_id = parse_u256(&payload.proposal_id, "proposal_id")?;
    let asset_type_id = parse_bytes32_input(&payload.asset_type_id, "asset_type_id")?;
    let max_supply = parse_u256(&payload.max_supply, "max_supply")?;
    let catalog_slug = build_catalog_slug(payload.slug.as_deref(), &payload.name)?;
    ensure_asset_creation_preconditions(state, proposal_id, asset_type_id, &catalog_slug).await?;
    let config_data = build_asset_creation_data(&payload)?;

    let tx_hash = send_factory_transaction::<_, Address>(
        &state.env,
        "createAsset",
        (
            proposal_id,
            asset_type_id,
            payload.name.clone(),
            payload.symbol.clone(),
            max_supply,
            config_data,
        ),
        "failed to submit createAsset transaction",
    )
    .await?;

    let asset_address = read_factory_asset_address(&state.env, proposal_id).await?;
    if asset_address == Address::zero() {
        return Err(AuthError::internal(
            "asset factory returned zero asset address after createAsset",
            anyhow!(
                "asset address not found for proposal {}",
                payload.proposal_id
            ),
        ));
    }

    let record = sync_asset(
        state,
        asset_address,
        Some(actor_user_id),
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    record_asset_price_history(
        state,
        &record,
        "create_asset",
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    upsert_asset_catalog(
        state,
        &record.asset_address,
        Some(actor_user_id),
        Some(actor_user_id),
        catalog_slug,
        payload.image_url.as_deref(),
        payload.summary.as_deref(),
        payload.market_segment.as_deref(),
        &payload.suggested_internal_tags,
        &payload.sources,
        payload.featured,
        payload.visible,
        payload.searchable,
    )
    .await?;

    let record = crud::get_asset(&state.db, state.env.monad_chain_id, &record.asset_address)
        .await?
        .ok_or_else(|| {
            AuthError::internal("asset missing after catalog update", "missing asset")
        })?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(record),
    })
}

pub async fn list_assets(
    state: &AppState,
    query: ListAssetsQuery,
) -> Result<AssetListResponse, AuthError> {
    let normalized = normalize_list_assets_query(query)?;
    list_assets_from_db(state, &normalized).await
}

pub async fn list_assets_by_type(
    state: &AppState,
    asset_type_id: &str,
) -> Result<AssetListResponse, AuthError> {
    let asset_type_id = parse_bytes32_input(asset_type_id, "asset_type_id")?;
    let normalized = NormalizedAssetListQuery {
        asset_type_id: Some(format_h256(asset_type_id)),
        q: None,
        asset_state: None,
        self_service_purchase_enabled: None,
        featured: None,
        limit: DEFAULT_LIST_LIMIT,
        offset: 0,
    };

    list_assets_from_db(state, &normalized).await
}

pub async fn get_asset_by_proposal(
    state: &AppState,
    proposal_id: &str,
) -> Result<AssetResponse, AuthError> {
    let proposal_id_u256 = parse_u256(proposal_id, "proposal_id")?;
    if let Some(record) =
        crud::get_asset_by_proposal(&state.db, state.env.monad_chain_id, proposal_id).await?
    {
        return Ok(AssetResponse::from(record));
    }

    let asset_address = read_factory_asset_address(&state.env, proposal_id_u256).await?;
    if asset_address == Address::zero() {
        return Err(AuthError::not_found("asset not found for proposal"));
    }

    Ok(AssetResponse::from(
        sync_asset(state, asset_address, None, None, None).await?,
    ))
}

pub async fn get_asset(state: &AppState, asset_address: &str) -> Result<AssetResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let asset_address_string = format_address(asset_address);

    match crud::get_asset(&state.db, state.env.monad_chain_id, &asset_address_string).await? {
        Some(record) => Ok(AssetResponse::from(record)),
        None => Ok(AssetResponse::from(
            sync_asset(state, asset_address, None, None, None).await?,
        )),
    }
}

pub async fn sync_asset_snapshot(
    state: &AppState,
    asset_address: &str,
    actor_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<AssetResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;

    Ok(AssetResponse::from(
        sync_asset(state, asset_address, None, actor_user_id, last_tx_hash).await?,
    ))
}

pub async fn get_asset_detail(
    state: &AppState,
    asset_address: &str,
    query: AssetDetailQuery,
) -> Result<AssetDetailResponse, AuthError> {
    let asset = get_asset(state, asset_address).await?;
    build_asset_detail(state, asset, query).await
}

pub async fn get_asset_by_slug(state: &AppState, slug: &str) -> Result<AssetResponse, AuthError> {
    let slug = normalize_slug(slug, "asset slug")?;

    match crud::get_asset_by_slug(&state.db, state.env.monad_chain_id, &slug).await? {
        Some(record) => Ok(AssetResponse::from(record)),
        None => Err(AuthError::not_found("asset not found")),
    }
}

pub async fn get_asset_detail_by_slug(
    state: &AppState,
    slug: &str,
    query: AssetDetailQuery,
) -> Result<AssetDetailResponse, AuthError> {
    let asset = get_asset_by_slug(state, slug).await?;
    build_asset_detail(state, asset, query).await
}

pub async fn get_asset_detail_by_proposal(
    state: &AppState,
    proposal_id: &str,
    query: AssetDetailQuery,
) -> Result<AssetDetailResponse, AuthError> {
    let asset = get_asset_by_proposal(state, proposal_id).await?;
    build_asset_detail(state, asset, query).await
}

pub async fn get_asset_history(
    state: &AppState,
    asset_address: &str,
    query: AssetHistoryQuery,
) -> Result<AssetHistoryResponse, AuthError> {
    let asset = get_asset(state, asset_address).await?;
    build_asset_history(state, asset, query).await
}

pub async fn get_asset_history_by_slug(
    state: &AppState,
    slug: &str,
    query: AssetHistoryQuery,
) -> Result<AssetHistoryResponse, AuthError> {
    let asset = get_asset_by_slug(state, slug).await?;
    build_asset_history(state, asset, query).await
}

pub async fn get_asset_history_by_proposal(
    state: &AppState,
    proposal_id: &str,
    query: AssetHistoryQuery,
) -> Result<AssetHistoryResponse, AuthError> {
    let asset = get_asset_by_proposal(state, proposal_id).await?;
    build_asset_history(state, asset, query).await
}

pub async fn get_asset_holder_state(
    state: &AppState,
    asset_address: &str,
    wallet_address: &str,
) -> Result<AssetHolderStateResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let wallet_address = parse_address(wallet_address)?;

    let asset_snapshot = read_asset_snapshot_from_chain(&state.env, asset_address).await?;
    let holder_snapshot =
        read_asset_holder_snapshot_from_chain(&state.env, asset_address, wallet_address).await?;

    Ok(asset_holder_response(
        &asset_snapshot.asset_address,
        holder_snapshot,
    ))
}

async fn build_asset_detail(
    state: &AppState,
    asset: AssetResponse,
    query: AssetDetailQuery,
) -> Result<AssetDetailResponse, AuthError> {
    if let Some(wallet_address) = query.wallet_address.as_deref() {
        parse_address(wallet_address)?;
    }

    let asset_address = asset.asset_address.clone();
    let holder_wallet_address = query.wallet_address.clone();

    let treasury_future = optional_detail_section(
        "treasury",
        treasury::get_treasury_asset(state, &asset_address),
    );
    let compliance_future = optional_detail_section(
        "compliance_rules",
        compliance::get_asset_rules(state, &asset_address),
    );
    let valuation_future =
        optional_detail_section("valuation", oracle::get_valuation(state, &asset_address));
    let holder_future = async {
        match holder_wallet_address.as_deref() {
            Some(wallet_address) => {
                optional_detail_section(
                    "holder",
                    get_asset_holder_state(state, &asset_address, wallet_address),
                )
                .await
            }
            None => (None, None),
        }
    };

    let (
        (treasury, treasury_unavailable),
        (compliance_rules, compliance_unavailable),
        (valuation, valuation_unavailable),
        (holder, holder_unavailable),
    ) = tokio::join!(
        treasury_future,
        compliance_future,
        valuation_future,
        holder_future
    );

    let unavailable_sections = [
        treasury_unavailable,
        compliance_unavailable,
        valuation_unavailable,
        holder_unavailable,
    ]
    .into_iter()
    .flatten()
    .collect();

    Ok(AssetDetailResponse {
        asset,
        treasury,
        compliance_rules,
        valuation,
        holder,
        unavailable_sections,
    })
}

async fn build_asset_history(
    state: &AppState,
    asset: AssetResponse,
    query: AssetHistoryQuery,
) -> Result<AssetHistoryResponse, AuthError> {
    let window = normalize_asset_history_range(query.range.as_deref())?;
    let observed_from = window.observed_from;
    let asset_address = asset.asset_address.clone();

    let (primary_history, valuation_history) = tokio::try_join!(
        crud::list_asset_price_history(&state.db, &asset_address, observed_from),
        oracle_crud::list_valuation_history(&state.db, &asset_address, observed_from),
    )?;

    let mut primary_samples = build_primary_history_samples(&primary_history)?;
    if primary_samples.is_empty() {
        primary_samples.push(HistorySample {
            observed_at: asset.updated_at,
            value: decimal_from_history_value(&asset.price_per_token, "asset price_per_token")?,
        });
    }

    let mut underlying_samples = build_underlying_history_samples(&valuation_history)?;
    if underlying_samples.is_empty() {
        if let Some(valuation) = oracle_crud::get_valuation(&state.db, &asset_address).await? {
            underlying_samples.push(HistorySample {
                observed_at: timestamp_seconds_to_utc(valuation.onchain_updated_at)?,
                value: decimal_from_history_value(
                    &valuation.nav_per_token,
                    "oracle valuation nav_per_token",
                )?,
            });
        }
    }

    let primary_market_price = build_history_candles(&primary_samples, window)?;
    let underlying_market_price = build_history_candles(&underlying_samples, window)?;
    let last_updated_at = [
        primary_market_price.last().map(|candle| candle.timestamp),
        underlying_market_price
            .last()
            .map(|candle| candle.timestamp),
    ]
    .into_iter()
    .flatten()
    .max();

    Ok(AssetHistoryResponse {
        asset_address,
        range: window.range.to_owned(),
        interval: window.interval_label.to_owned(),
        last_updated_at,
        primary_market_price,
        underlying_market_price,
    })
}

pub async fn preview_purchase(
    state: &AppState,
    asset_address: &str,
    payload: AssetPreviewRequest,
) -> Result<AssetPreviewResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let token_amount = parse_u256(&payload.token_amount, "token_amount")?;
    let value = read_asset_preview_purchase(&state.env, asset_address, token_amount).await?;

    Ok(AssetPreviewResponse {
        asset_address: format_address(asset_address),
        token_amount: token_amount.to_string(),
        value: value.to_string(),
    })
}

pub async fn preview_redemption(
    state: &AppState,
    asset_address: &str,
    payload: AssetPreviewRequest,
) -> Result<AssetPreviewResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let token_amount = parse_u256(&payload.token_amount, "token_amount")?;
    let value = read_asset_preview_redemption(&state.env, asset_address, token_amount).await?;

    Ok(AssetPreviewResponse {
        asset_address: format_address(asset_address),
        token_amount: token_amount.to_string(),
        value: value.to_string(),
    })
}

pub async fn check_transfer(
    state: &AppState,
    asset_address: &str,
    payload: crate::module::asset::schema::AssetCheckTransferRequest,
) -> Result<AssetTransferCheckResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let from_wallet = parse_address(&payload.from_wallet)?;
    let to_wallet = parse_address(&payload.to_wallet)?;
    let amount = parse_u256(&payload.amount, "amount")?;
    let data = parse_bytes_input(payload.data.as_deref(), "data")?;

    let (status_code, reason_code) = read_asset_can_transfer(
        &state.env,
        asset_address,
        from_wallet,
        to_wallet,
        amount,
        data,
    )
    .await?;

    Ok(AssetTransferCheckResponse {
        status_code: format!("0x{:02x}", status_code[0]),
        reason_code: format_h256(reason_code),
        reason: bytes32_reason(reason_code),
    })
}

pub async fn issue_asset(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminIssueAssetRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let recipient_wallet = parse_address(&payload.recipient_wallet)?;
    let amount = parse_u256(&payload.amount, "amount")?;
    let data = parse_bytes_input(payload.data.as_deref(), "data")?;

    let tx_hash = send_asset_transaction::<_, bool>(
        &state.env,
        asset_address,
        "issue",
        (recipient_wallet, amount, data),
        "failed to submit issue transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    record_asset_price_history(
        state,
        &asset,
        "set_subscription_price",
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn burn_asset(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminBurnAssetRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let from_wallet = parse_address(&payload.from_wallet)?;
    let amount = parse_u256(&payload.amount, "amount")?;

    let tx_hash = send_asset_transaction::<_, bool>(
        &state.env,
        asset_address,
        "burn",
        (from_wallet, amount),
        "failed to submit burn transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    record_asset_price_history(
        state,
        &asset,
        "set_redemption_price",
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn set_asset_state(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetStateRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let state_value = parse_asset_state(&payload.state)?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "setAssetState",
        state_value,
        "failed to submit setAssetState transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    record_asset_price_history(
        state,
        &asset,
        "set_pricing",
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn archive_asset(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
) -> Result<AssetArchiveWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let mut asset_record =
        load_asset_record_for_admin_write(state, asset_address, actor_user_id).await?;
    let mut state_tx_hash = None;
    let mut self_service_purchase_tx_hash = None;

    if asset_record.asset_state != 1 {
        let tx_hash = send_asset_transaction::<_, ()>(
            &state.env,
            asset_address,
            "setAssetState",
            1_u8,
            "failed to submit archive setAssetState transaction",
        )
        .await?;
        asset_record = sync_asset(
            state,
            asset_address,
            None,
            Some(actor_user_id),
            Some(&tx_hash),
        )
        .await?;
        state_tx_hash = Some(tx_hash);
    }

    if asset_record.self_service_purchase_enabled {
        let tx_hash = send_asset_transaction::<_, ()>(
            &state.env,
            asset_address,
            "setSelfServicePurchaseEnabled",
            false,
            "failed to submit archive setSelfServicePurchaseEnabled transaction",
        )
        .await?;
        asset_record = sync_asset(
            state,
            asset_address,
            None,
            Some(actor_user_id),
            Some(&tx_hash),
        )
        .await?;
        self_service_purchase_tx_hash = Some(tx_hash);
    }

    let slug = build_catalog_slug(asset_record.slug.as_deref(), &asset_record.name)?;
    upsert_asset_catalog(
        state,
        &asset_record.asset_address,
        asset_record.created_by_user_id.or(Some(actor_user_id)),
        Some(actor_user_id),
        slug,
        asset_record.image_url.as_deref(),
        asset_record.summary.as_deref(),
        asset_record.market_segment.as_deref(),
        &asset_record.suggested_internal_tags,
        &asset_record.sources,
        false,
        false,
        false,
    )
    .await?;

    let asset_record = crud::get_asset(
        &state.db,
        state.env.monad_chain_id,
        &asset_record.asset_address,
    )
    .await?
    .ok_or_else(|| AuthError::internal("asset missing after archive", "missing asset"))?;

    Ok(AssetArchiveWriteResponse {
        state_tx_hash,
        self_service_purchase_tx_hash,
        asset: AssetResponse::from(asset_record),
    })
}

pub async fn set_subscription_price(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetPriceRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let value = parse_u256(&payload.value, "value")?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "setPricePerToken",
        value,
        "failed to submit setPricePerToken transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn set_redemption_price(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetPriceRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let value = parse_u256(&payload.value, "value")?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "setRedemptionPricePerToken",
        value,
        "failed to submit setRedemptionPricePerToken transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn set_pricing(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetPricingRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let subscription_price = parse_u256(&payload.subscription_price, "subscription_price")?;
    let redemption_price = parse_u256(&payload.redemption_price, "redemption_price")?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "setPricing",
        (subscription_price, redemption_price),
        "failed to submit setPricing transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn set_self_service_purchase_enabled(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetSelfServicePurchaseRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "setSelfServicePurchaseEnabled",
        payload.enabled,
        "failed to submit setSelfServicePurchaseEnabled transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn set_metadata_hash(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetMetadataRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let metadata_hash = parse_bytes32_input(&payload.metadata_hash, "metadata_hash")?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "setMetadataHash",
        metadata_hash,
        "failed to submit setMetadataHash transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn set_asset_catalog(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetCatalogRequest,
) -> Result<AssetCatalogWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let asset_record = match sync_asset(state, asset_address, None, Some(actor_user_id), None).await
    {
        Ok(record) => record,
        Err(error) => match crud::get_asset(
            &state.db,
            state.env.monad_chain_id,
            &format_address(asset_address),
        )
        .await?
        {
            Some(record) => record,
            None => return Err(error),
        },
    };

    upsert_asset_catalog(
        state,
        &asset_record.asset_address,
        asset_record.created_by_user_id.or(Some(actor_user_id)),
        Some(actor_user_id),
        normalize_slug(&payload.slug, "asset slug")?,
        payload.image_url.as_deref(),
        payload.summary.as_deref(),
        payload.market_segment.as_deref(),
        &payload.suggested_internal_tags,
        &payload.sources,
        payload.featured,
        payload.visible,
        payload.searchable,
    )
    .await?;

    let record = crud::get_asset(
        &state.db,
        state.env.monad_chain_id,
        &asset_record.asset_address,
    )
    .await?
    .ok_or_else(|| AuthError::internal("asset missing after catalog update", "missing asset"))?;

    Ok(AssetCatalogWriteResponse::from_record(record))
}

pub async fn set_compliance_registry(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetComplianceRegistryRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let compliance_registry_address = parse_address(&payload.compliance_registry_address)?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "setComplianceRegistry",
        compliance_registry_address,
        "failed to submit setComplianceRegistry transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn set_treasury(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetAssetTreasuryRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let treasury_address = parse_address(&payload.treasury_address)?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "setTreasury",
        treasury_address,
        "failed to submit setTreasury transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn disable_controller(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "disableController",
        (),
        "failed to submit disableController transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn controller_transfer(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminControllerTransferRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let from_wallet = parse_address(&payload.from_wallet)?;
    let to_wallet = parse_address(&payload.to_wallet)?;
    let amount = parse_u256(&payload.amount, "amount")?;
    let data = parse_bytes_input(payload.data.as_deref(), "data")?;
    let operator_data = parse_bytes_input(payload.operator_data.as_deref(), "operator_data")?;

    let tx_hash = send_asset_transaction::<_, ()>(
        &state.env,
        asset_address,
        "controllerTransfer",
        (from_wallet, to_wallet, amount, data, operator_data),
        "failed to submit controllerTransfer transaction",
    )
    .await?;

    let asset = sync_asset(
        state,
        asset_address,
        None,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AssetWriteResponse {
        tx_hash,
        asset: AssetResponse::from(asset),
    })
}

pub async fn process_redemption(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminProcessRedemptionRequest,
) -> Result<AssetWriteResponse, AuthError> {
    let _ = state;
    let _ = actor_user_id;
    let _ = asset_address;
    let _ = payload;
    Err(AuthError::bad_request(
        "manual redemption processing is disabled in the deployed asset token; use /assets/{asset_address}/redeem instead",
    ))
}

pub async fn approve_payment_token(
    state: &AppState,
    user_id: Uuid,
    asset_address: &str,
    payload: GaslessApprovePaymentTokenRequest,
) -> Result<GaslessAssetActionResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let amount = parse_u256(&payload.amount, "amount")?;
    let wallet = user_wallet_for_action(&state.db, user_id).await?;
    let asset = read_asset_snapshot_from_chain(&state.env, asset_address).await?;
    let call_data = build_erc20_calldata::<_, bool>(
        &state.env,
        parse_address(&asset.payment_token_address)?,
        "approve",
        (parse_address(&asset.treasury_address)?, amount),
    )
    .await?;
    let tx_hash = gasless::submit_user_calls(
        state,
        user_id,
        vec![
            gasless::target_call(parse_address(&asset.payment_token_address)?, call_data).map_err(
                |error| AuthError::internal("failed to build payment token approval call", error),
            )?,
        ],
    )
    .await?;

    let asset_record = sync_asset(state, asset_address, None, None, Some(&tx_hash)).await?;
    let holder = read_asset_holder_snapshot_from_chain(&state.env, asset_address, wallet).await?;

    Ok(GaslessAssetActionResponse {
        tx_hash,
        asset: AssetResponse::from(asset_record),
        holder: asset_holder_response(&format_address(asset_address), holder),
    })
}

pub async fn purchase_asset(
    state: &AppState,
    user_id: Uuid,
    asset_address: &str,
    payload: GaslessPurchaseAssetRequest,
) -> Result<GaslessAssetActionResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let token_amount = parse_u256(&payload.token_amount, "token_amount")?;
    let wallet = user_wallet_for_action(&state.db, user_id).await?;
    let asset = read_asset_snapshot_from_chain(&state.env, asset_address).await?;
    let holder = read_asset_holder_snapshot_from_chain(&state.env, asset_address, wallet).await?;
    let payment_token_cost =
        read_asset_preview_purchase(&state.env, asset_address, token_amount).await?;

    tracing::info!(
        asset_address = %format_address(asset_address),
        wallet_address = %format_address(wallet),
        token_amount = %token_amount,
        payment_token_cost = %payment_token_cost,
        total_supply = %asset.total_supply,
        max_supply = %asset.max_supply,
        payment_token_balance = %holder.payment_token_balance,
        payment_token_allowance_to_treasury = %holder.payment_token_allowance_to_treasury,
        asset_state = asset.asset_state,
        asset_state_label = %asset.asset_state_label,
        self_service_purchase_enabled = asset.self_service_purchase_enabled,
        "asset purchase preflight"
    );

    ensure_purchase_preconditions(
        state,
        wallet,
        &asset,
        &holder,
        token_amount,
        payment_token_cost,
    )
    .await?;

    let call_data =
        build_asset_calldata::<_, ()>(&state.env, asset_address, "purchase", token_amount).await?;
    let tx_hash = gasless::submit_user_calls(
        state,
        user_id,
        vec![
            gasless::target_call(asset_address, call_data)
                .map_err(|error| AuthError::internal("failed to build purchase call", error))?,
        ],
    )
    .await
    .map_err(|error| {
        tracing::error!(
            asset_address = %format_address(asset_address),
            wallet_address = %format_address(wallet),
            token_amount = %token_amount,
            payment_token_cost = %payment_token_cost,
            payment_token_balance = %holder.payment_token_balance,
            payment_token_allowance_to_treasury = %holder.payment_token_allowance_to_treasury,
            error = %error,
            "asset purchase submission failed"
        );
        error
    })?;

    let asset_record = sync_asset(state, asset_address, None, None, Some(&tx_hash)).await?;
    let holder = read_asset_holder_snapshot_from_chain(&state.env, asset_address, wallet).await?;

    // Log trade history
    let _ = crud::insert_trade_history(
        &state.db,
        user_id,
        &format_address(wallet),
        &format_address(asset_address),
        "purchase",
        &token_amount.to_string(),
        &payment_token_cost.to_string(),
        &asset_record.price_per_token,
        &tx_hash,
    )
    .await;

    Ok(GaslessAssetActionResponse {
        tx_hash,
        asset: AssetResponse::from(asset_record),
        holder: asset_holder_response(&format_address(asset_address), holder),
    })
}

pub async fn claim_yield(
    state: &AppState,
    user_id: Uuid,
    asset_address: &str,
    payload: GaslessClaimYieldRequest,
) -> Result<GaslessAssetActionResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let wallet = user_wallet_for_action(&state.db, user_id).await?;
    let recipient = match payload.recipient_wallet {
        Some(value) => parse_address(&value)?,
        None => wallet,
    };
    let call_data =
        build_asset_calldata::<_, U256>(&state.env, asset_address, "claimYield", recipient).await?;
    let tx_hash = gasless::submit_user_calls(
        state,
        user_id,
        vec![
            gasless::target_call(asset_address, call_data)
                .map_err(|error| AuthError::internal("failed to build claimYield call", error))?,
        ],
    )
    .await?;

    let asset_record = sync_asset(state, asset_address, None, None, Some(&tx_hash)).await?;
    let holder = read_asset_holder_snapshot_from_chain(&state.env, asset_address, wallet).await?;

    Ok(GaslessAssetActionResponse {
        tx_hash,
        asset: AssetResponse::from(asset_record),
        holder: asset_holder_response(&format_address(asset_address), holder),
    })
}

pub async fn redeem_asset(
    state: &AppState,
    user_id: Uuid,
    asset_address: &str,
    payload: GaslessRedeemAssetRequest,
) -> Result<GaslessAssetActionResponse, AuthError> {
    let asset_address = parse_address(asset_address)?;
    let wallet = user_wallet_for_action(&state.db, user_id).await?;
    let amount = parse_u256(&payload.amount, "amount")?;
    let data = parse_bytes_input(payload.data.as_deref(), "data")?;
    let call_data =
        build_asset_calldata::<_, U256>(&state.env, asset_address, "redeem", (amount, data))
            .await?;
    let tx_hash = gasless::submit_user_calls(
        state,
        user_id,
        vec![
            gasless::target_call(asset_address, call_data)
                .map_err(|error| AuthError::internal("failed to build redeem call", error))?,
        ],
    )
    .await?;

    let asset_record = sync_asset(state, asset_address, None, None, Some(&tx_hash)).await?;
    let holder = read_asset_holder_snapshot_from_chain(&state.env, asset_address, wallet).await?;

    // Calculate payment amount for history
    let redemption_value = read_asset_preview_redemption(&state.env, asset_address, amount).await?;

    // Log trade history
    let _ = crud::insert_trade_history(
        &state.db,
        user_id,
        &format_address(wallet),
        &format_address(asset_address),
        "redemption",
        &amount.to_string(),
        &redemption_value.to_string(),
        &asset_record.redemption_price_per_token,
        &tx_hash,
    )
    .await;

    Ok(GaslessAssetActionResponse {
        tx_hash,
        asset: AssetResponse::from(asset_record),
        holder: asset_holder_response(&format_address(asset_address), holder),
    })
}

pub async fn cancel_redemption(
    state: &AppState,
    user_id: Uuid,
    asset_address: &str,
    payload: GaslessCancelRedemptionRequest,
) -> Result<GaslessAssetActionResponse, AuthError> {
    let _ = state;
    let _ = user_id;
    let _ = asset_address;
    let _ = payload;
    Err(AuthError::bad_request(
        "manual redemption cancellation is disabled in the deployed asset token; redemptions settle atomically during /assets/{asset_address}/redeem",
    ))
}

async fn sync_asset_type(
    state: &AppState,
    asset_type_id: H256,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<AssetTypeRecord, AuthError> {
    let (asset_type_name, implementation_address, is_registered) =
        read_asset_type_from_chain(&state.env, asset_type_id).await?;

    crud::upsert_asset_type(
        &state.db,
        &format_h256(asset_type_id),
        &asset_type_name,
        &format_address(implementation_address),
        is_registered,
        updated_by_user_id,
        last_tx_hash,
    )
    .await
}

async fn sync_asset(
    state: &AppState,
    asset_address: Address,
    created_by_user_id: Option<Uuid>,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<AssetRecord, AuthError> {
    let snapshot = read_asset_snapshot_from_chain(&state.env, asset_address).await?;

    let record = crud::upsert_asset(
        &state.db,
        &snapshot.asset_address,
        state.env.monad_chain_id,
        &snapshot.proposal_id,
        &snapshot.asset_type_id,
        &snapshot.name,
        &snapshot.symbol,
        &snapshot.max_supply,
        &snapshot.total_supply,
        snapshot.asset_state,
        &snapshot.asset_state_label,
        snapshot.controllable,
        snapshot.self_service_purchase_enabled,
        &snapshot.price_per_token,
        &snapshot.redemption_price_per_token,
        &snapshot.treasury_address,
        &snapshot.compliance_registry_address,
        &snapshot.payment_token_address,
        &snapshot.metadata_hash,
        &snapshot.holder_count,
        &snapshot.total_pending_redemptions,
        created_by_user_id,
        updated_by_user_id,
        last_tx_hash,
    )
    .await?;

    match crud::get_asset(&state.db, state.env.monad_chain_id, &record.asset_address).await? {
        Some(record) => Ok(record),
        None => Ok(record),
    }
}

async fn user_wallet_for_action(
    db: &crate::config::db::DbPool,
    user_id: Uuid,
) -> Result<Address, AuthError> {
    let wallet = auth_crud::get_wallet_for_user(db, user_id)
        .await?
        .ok_or_else(|| AuthError::forbidden("user wallet is not linked"))?;
    parse_address(&wallet.wallet_address)
}

async fn ensure_purchase_preconditions(
    state: &AppState,
    wallet: Address,
    asset: &AssetSnapshot,
    holder: &AssetHolderSnapshot,
    token_amount: U256,
    payment_token_cost: U256,
) -> Result<(), AuthError> {
    if asset.asset_state != 0 {
        return Err(AuthError::bad_request(format!(
            "asset is not active for purchase: current_state={}",
            asset.asset_state_label
        )));
    }

    if !asset.self_service_purchase_enabled {
        return Err(AuthError::bad_request(
            "self-service purchase is disabled for this asset",
        ));
    }

    let payment_token_balance = U256::from_dec_str(&holder.payment_token_balance)
        .map_err(|error| AuthError::internal("invalid payment token balance snapshot", error))?;
    if payment_token_balance < payment_token_cost {
        return Err(AuthError::bad_request(format!(
            "insufficient payment-token balance for purchase: required={}, available={}",
            payment_token_cost, payment_token_balance
        )));
    }

    let payment_token_allowance = U256::from_dec_str(&holder.payment_token_allowance_to_treasury)
        .map_err(|error| {
        AuthError::internal("invalid payment token allowance snapshot", error)
    })?;
    if payment_token_allowance < payment_token_cost {
        return Err(AuthError::bad_request(format!(
            "insufficient payment-token allowance to treasury for purchase: required={}, approved={}",
            payment_token_cost, payment_token_allowance
        )));
    }

    let current_total_supply = U256::from_dec_str(&asset.total_supply)
        .map_err(|error| AuthError::internal("invalid asset total supply snapshot", error))?;
    let max_supply = U256::from_dec_str(&asset.max_supply)
        .map_err(|error| AuthError::internal("invalid asset max supply snapshot", error))?;
    let projected_total_supply = current_total_supply.saturating_add(token_amount);
    if projected_total_supply > max_supply {
        return Err(AuthError::bad_request(format!(
            "purchase exceeds asset max supply: projected_total_supply={}, max_supply={}",
            projected_total_supply, max_supply
        )));
    }

    let resulting_balance = U256::from_dec_str(&holder.balance)
        .map_err(|error| AuthError::internal("invalid asset balance snapshot", error))?
        .saturating_add(token_amount);
    let wallet_address = format_address(wallet);
    let asset_address = asset.asset_address.clone();
    let amount = token_amount.to_string();
    let resulting_balance_string = resulting_balance.to_string();
    let mut compliance = compliance::check_subscribe(
        state,
        ComplianceCheckSubscribeRequest {
            asset_address: asset_address.clone(),
            investor_wallet: wallet_address.clone(),
            amount: amount.clone(),
            resulting_balance: resulting_balance_string.clone(),
        },
    )
    .await?;
    if !compliance.is_valid
        && state.env.open_purchase_auto_whitelist
        && compliance.reason == "INVESTOR_NOT_ELIGIBLE"
    {
        tracing::info!(
            asset_address,
            wallet_address,
            "auto-whitelisting investor for open purchase"
        );
        let investor =
            compliance::auto_allow_investor_for_open_purchase(state, &wallet_address).await?;
        tracing::info!(
            wallet_address = %investor.wallet_address,
            is_verified = investor.is_verified,
            is_whitelisted = investor.is_whitelisted,
            "open purchase auto-whitelist applied"
        );
        compliance = compliance::check_subscribe(
            state,
            ComplianceCheckSubscribeRequest {
                asset_address,
                investor_wallet: wallet_address,
                amount,
                resulting_balance: resulting_balance_string,
            },
        )
        .await?;
    }
    if !compliance.is_valid {
        return Err(AuthError::bad_request(format!(
            "purchase blocked by compliance: {}",
            compliance.reason
        )));
    }

    tracing::debug!(
        token_amount = %token_amount,
        payment_token_cost = %payment_token_cost,
        total_supply = %current_total_supply,
        max_supply = %max_supply,
        projected_total_supply = %projected_total_supply,
        payment_token_balance = %payment_token_balance,
        payment_token_allowance = %payment_token_allowance,
        resulting_balance = %resulting_balance,
        "asset purchase preflight passed"
    );

    Ok(())
}

async fn read_factory_status_from_chain(
    env: &Environment,
) -> Result<AssetFactorySnapshot, AuthError> {
    let contract = read_factory_contract(env).await.map_err(|error| {
        AuthError::internal("failed to build asset factory read contract", error)
    })?;

    let access_control_address = contract
        .method::<_, Address>("accessControl", ())
        .map_err(|error| AuthError::internal("failed to build accessControl call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call accessControl", error))?;
    let compliance_registry_address = contract
        .method::<_, Address>("complianceRegistry", ())
        .map_err(|error| AuthError::internal("failed to build complianceRegistry call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call complianceRegistry", error))?;
    let treasury_address = contract
        .method::<_, Address>("treasury", ())
        .map_err(|error| AuthError::internal("failed to build treasury call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call treasury", error))?;
    let paused = contract
        .method::<_, bool>("paused", ())
        .map_err(|error| AuthError::internal("failed to build paused call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call paused", error))?;
    let total_assets_created = contract
        .method::<_, U256>("getTotalAssetsCreated", ())
        .map_err(|error| AuthError::internal("failed to build getTotalAssetsCreated call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getTotalAssetsCreated", error))?;

    Ok(AssetFactorySnapshot {
        access_control_address: format_address(access_control_address),
        compliance_registry_address: format_address(compliance_registry_address),
        treasury_address: format_address(treasury_address),
        paused,
        total_assets_created: total_assets_created.to_string(),
    })
}

async fn read_registered_asset_type_ids(env: &Environment) -> Result<Vec<H256>, AuthError> {
    let contract = read_factory_contract(env).await.map_err(|error| {
        AuthError::internal("failed to build asset factory read contract", error)
    })?;

    contract
        .method::<_, Vec<H256>>("getAllRegisteredAssetTypes", ())
        .map_err(|error| {
            AuthError::internal("failed to build getAllRegisteredAssetTypes call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getAllRegisteredAssetTypes", error))
}

async fn read_asset_type_from_chain(
    env: &Environment,
    asset_type_id: H256,
) -> Result<(String, Address, bool), AuthError> {
    let contract = read_factory_contract(env).await.map_err(|error| {
        AuthError::internal("failed to build asset factory read contract", error)
    })?;

    let asset_type_name = contract
        .method::<_, String>("getAssetTypeName", asset_type_id)
        .map_err(|error| AuthError::internal("failed to build getAssetTypeName call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getAssetTypeName", error))?;
    let implementation_address = contract
        .method::<_, Address>("getAssetTypeImplementation", asset_type_id)
        .map_err(|error| {
            AuthError::internal("failed to build getAssetTypeImplementation call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getAssetTypeImplementation", error))?;
    let is_registered = contract
        .method::<_, bool>("isAssetTypeRegistered", asset_type_id)
        .map_err(|error| AuthError::internal("failed to build isAssetTypeRegistered call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call isAssetTypeRegistered", error))?;

    Ok((asset_type_name, implementation_address, is_registered))
}

async fn read_factory_asset_address(
    env: &Environment,
    proposal_id: U256,
) -> Result<Address, AuthError> {
    let contract = read_factory_contract(env).await.map_err(|error| {
        AuthError::internal("failed to build asset factory read contract", error)
    })?;

    contract
        .method::<_, Address>("getAssetAddress", proposal_id)
        .map_err(|error| AuthError::internal("failed to build getAssetAddress call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getAssetAddress", error))
}

async fn read_asset_snapshot_from_chain(
    env: &Environment,
    asset_address: Address,
) -> Result<AssetSnapshot, AuthError> {
    let contract = read_asset_contract(env, asset_address)
        .await
        .map_err(|error| AuthError::internal("failed to build asset read contract", error))?;

    let proposal_id = contract
        .method::<_, U256>("getProposalId", ())
        .map_err(|error| AuthError::internal("failed to build getProposalId call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getProposalId", error))?;
    let asset_type_id = contract
        .method::<_, H256>("getAssetTypeId", ())
        .map_err(|error| AuthError::internal("failed to build getAssetTypeId call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getAssetTypeId", error))?;
    let name = contract
        .method::<_, String>("name", ())
        .map_err(|error| AuthError::internal("failed to build name call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call name", error))?;
    let symbol = contract
        .method::<_, String>("symbol", ())
        .map_err(|error| AuthError::internal("failed to build symbol call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call symbol", error))?;
    let max_supply = contract
        .method::<_, U256>("maxSupply", ())
        .map_err(|error| AuthError::internal("failed to build maxSupply call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call maxSupply", error))?;
    let total_supply = contract
        .method::<_, U256>("totalSupply", ())
        .map_err(|error| AuthError::internal("failed to build totalSupply call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call totalSupply", error))?;
    let asset_state = contract
        .method::<_, u8>("getAssetState", ())
        .map_err(|error| AuthError::internal("failed to build getAssetState call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getAssetState", error))?;
    let controllable = contract
        .method::<_, bool>("isControllable", ())
        .map_err(|error| AuthError::internal("failed to build isControllable call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call isControllable", error))?;
    let self_service_purchase_enabled = contract
        .method::<_, bool>("selfServicePurchaseEnabled", ())
        .map_err(|error| {
            AuthError::internal("failed to build selfServicePurchaseEnabled call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call selfServicePurchaseEnabled", error))?;
    let price_per_token = contract
        .method::<_, U256>("pricePerToken", ())
        .map_err(|error| AuthError::internal("failed to build pricePerToken call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call pricePerToken", error))?;
    let redemption_price_per_token = contract
        .method::<_, U256>("redemptionPricePerToken", ())
        .map_err(|error| {
            AuthError::internal("failed to build redemptionPricePerToken call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call redemptionPricePerToken", error))?;
    let treasury_address = contract
        .method::<_, Address>("treasury", ())
        .map_err(|error| AuthError::internal("failed to build treasury call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call treasury", error))?;
    let compliance_registry_address = contract
        .method::<_, Address>("complianceRegistry", ())
        .map_err(|error| AuthError::internal("failed to build complianceRegistry call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call complianceRegistry", error))?;
    let payment_token_address = contract
        .method::<_, Address>("paymentToken", ())
        .map_err(|error| AuthError::internal("failed to build paymentToken call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call paymentToken", error))?;
    let metadata_hash = contract
        .method::<_, H256>("metadataHash", ())
        .map_err(|error| AuthError::internal("failed to build metadataHash call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call metadataHash", error))?;
    let holder_count = contract
        .method::<_, U256>("holderCount", ())
        .map_err(|error| AuthError::internal("failed to build holderCount call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call holderCount", error))?;
    let total_pending_redemptions = contract
        .method::<_, U256>("totalPendingRedemptions", ())
        .map_err(|error| {
            AuthError::internal("failed to build totalPendingRedemptions call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call totalPendingRedemptions", error))?;

    Ok(AssetSnapshot {
        asset_address: format_address(asset_address),
        proposal_id: proposal_id.to_string(),
        asset_type_id: format_h256(asset_type_id),
        name,
        symbol,
        max_supply: max_supply.to_string(),
        total_supply: total_supply.to_string(),
        asset_state: i32::from(asset_state),
        asset_state_label: asset_state_label(asset_state).to_owned(),
        controllable,
        self_service_purchase_enabled,
        price_per_token: price_per_token.to_string(),
        redemption_price_per_token: redemption_price_per_token.to_string(),
        treasury_address: format_address(treasury_address),
        compliance_registry_address: format_address(compliance_registry_address),
        payment_token_address: format_address(payment_token_address),
        metadata_hash: format_h256(metadata_hash),
        holder_count: holder_count.to_string(),
        total_pending_redemptions: total_pending_redemptions.to_string(),
    })
}

async fn read_asset_holder_snapshot_from_chain(
    env: &Environment,
    asset_address: Address,
    wallet_address: Address,
) -> Result<AssetHolderSnapshot, AuthError> {
    let asset_contract = read_asset_contract(env, asset_address)
        .await
        .map_err(|error| AuthError::internal("failed to build asset read contract", error))?;

    let treasury_address = asset_contract
        .method::<_, Address>("treasury", ())
        .map_err(|error| AuthError::internal("failed to build treasury call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call treasury", error))?;
    let payment_token_address = asset_contract
        .method::<_, Address>("paymentToken", ())
        .map_err(|error| AuthError::internal("failed to build paymentToken call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call paymentToken", error))?;
    let balance = asset_contract
        .method::<_, U256>("balanceOf", wallet_address)
        .map_err(|error| AuthError::internal("failed to build balanceOf call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call balanceOf", error))?;
    let claimable_yield = asset_contract
        .method::<_, U256>("claimableYieldOf", wallet_address)
        .map_err(|error| AuthError::internal("failed to build claimableYieldOf call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call claimableYieldOf", error))?;
    let accumulative_yield = asset_contract
        .method::<_, U256>("accumulativeYieldOf", wallet_address)
        .map_err(|error| AuthError::internal("failed to build accumulativeYieldOf call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call accumulativeYieldOf", error))?;
    let pending_redemption = asset_contract
        .method::<_, U256>("pendingRedemptionOf", wallet_address)
        .map_err(|error| AuthError::internal("failed to build pendingRedemptionOf call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call pendingRedemptionOf", error))?;
    let locked_balance = asset_contract
        .method::<_, U256>("lockedBalanceOf", wallet_address)
        .map_err(|error| AuthError::internal("failed to build lockedBalanceOf call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call lockedBalanceOf", error))?;

    let payment_token_contract = read_erc20_contract(env, payment_token_address)
        .await
        .map_err(|error| {
            AuthError::internal("failed to build payment token read contract", error)
        })?;
    let payment_token_balance = payment_token_contract
        .method::<_, U256>("balanceOf", wallet_address)
        .map_err(|error| {
            AuthError::internal("failed to build payment token balanceOf call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call payment token balanceOf", error))?;
    let allowance = payment_token_contract
        .method::<_, U256>("allowance", (wallet_address, treasury_address))
        .map_err(|error| AuthError::internal("failed to build allowance call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call allowance", error))?;

    Ok(AssetHolderSnapshot {
        wallet_address: format_address(wallet_address),
        balance: balance.to_string(),
        claimable_yield: claimable_yield.to_string(),
        accumulative_yield: accumulative_yield.to_string(),
        pending_redemption: pending_redemption.to_string(),
        locked_balance: locked_balance.to_string(),
        unlocked_balance: balance.saturating_sub(locked_balance).to_string(),
        payment_token_balance: payment_token_balance.to_string(),
        payment_token_allowance_to_treasury: allowance.to_string(),
    })
}

async fn read_asset_preview_purchase(
    env: &Environment,
    asset_address: Address,
    token_amount: U256,
) -> Result<U256, AuthError> {
    let contract = read_asset_contract(env, asset_address)
        .await
        .map_err(|error| AuthError::internal("failed to build asset read contract", error))?;
    contract
        .method::<_, U256>("previewPurchase", token_amount)
        .map_err(|error| AuthError::internal("failed to build previewPurchase call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call previewPurchase", error))
}

async fn read_asset_preview_redemption(
    env: &Environment,
    asset_address: Address,
    token_amount: U256,
) -> Result<U256, AuthError> {
    let contract = read_asset_contract(env, asset_address)
        .await
        .map_err(|error| AuthError::internal("failed to build asset read contract", error))?;
    contract
        .method::<_, U256>("previewRedemption", token_amount)
        .map_err(|error| AuthError::internal("failed to build previewRedemption call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call previewRedemption", error))
}

async fn read_asset_can_transfer(
    env: &Environment,
    asset_address: Address,
    from_wallet: Address,
    to_wallet: Address,
    amount: U256,
    data: Bytes,
) -> Result<([u8; 1], H256), AuthError> {
    let contract = read_asset_contract(env, asset_address)
        .await
        .map_err(|error| AuthError::internal("failed to build asset read contract", error))?;
    contract
        .method::<_, ([u8; 1], H256)>("canTransfer", (to_wallet, amount, data))
        .map_err(|error| AuthError::internal("failed to build canTransfer call", error))?
        .from(from_wallet)
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call canTransfer", error))
}

fn build_asset_creation_data(payload: &AdminCreateAssetRequest) -> Result<Bytes, AuthError> {
    Ok(Bytes::from(encode(&[Token::Tuple(vec![
        Token::Uint(parse_u256(
            &payload.subscription_price,
            "subscription_price",
        )?),
        Token::Uint(parse_u256(&payload.redemption_price, "redemption_price")?),
        Token::Bool(payload.self_service_purchase_enabled),
        Token::FixedBytes(
            parse_bytes32_input(
                payload.metadata_hash.as_deref().unwrap_or_default(),
                "metadata_hash",
            )?
            .as_bytes()
            .to_vec(),
        ),
    ])])))
}

fn factory_status_response(
    env: &Environment,
    snapshot: AssetFactorySnapshot,
) -> AssetFactoryStatusResponse {
    AssetFactoryStatusResponse {
        factory_address: env.asset_factory_address.clone(),
        access_control_address: snapshot.access_control_address,
        compliance_diamond_address: snapshot.compliance_registry_address.clone(),
        compliance_registry_address: snapshot.compliance_registry_address,
        treasury_address: snapshot.treasury_address,
        paused: snapshot.paused,
        total_assets_created: snapshot.total_assets_created,
    }
}

fn asset_holder_response(
    asset_address: &str,
    snapshot: AssetHolderSnapshot,
) -> AssetHolderStateResponse {
    AssetHolderStateResponse {
        asset_address: asset_address.to_owned(),
        wallet_address: snapshot.wallet_address,
        balance: snapshot.balance,
        claimable_yield: snapshot.claimable_yield,
        accumulative_yield: snapshot.accumulative_yield,
        pending_redemption: snapshot.pending_redemption,
        locked_balance: snapshot.locked_balance,
        unlocked_balance: snapshot.unlocked_balance,
        payment_token_balance: snapshot.payment_token_balance,
        payment_token_allowance_to_treasury: snapshot.payment_token_allowance_to_treasury,
    }
}

async fn list_assets_from_db(
    state: &AppState,
    query: &NormalizedAssetListQuery,
) -> Result<AssetListResponse, AuthError> {
    let assets = crud::list_assets(
        &state.db,
        crud::AssetListFilters {
            chain_id: state.env.monad_chain_id,
            asset_type_id: query.asset_type_id.as_deref(),
            tag_slug: None,
            q: query.q.as_deref(),
            asset_state: query.asset_state,
            self_service_purchase_enabled: query.self_service_purchase_enabled,
            featured: query.featured,
            limit: query.limit,
            offset: query.offset,
            only_visible: true,
            require_searchable: query.q.is_some(),
        },
    )
    .await?;

    Ok(AssetListResponse::new(
        assets.into_iter().map(AssetResponse::from).collect(),
        query.limit,
        query.offset,
    ))
}

async fn upsert_asset_catalog(
    state: &AppState,
    asset_address: &str,
    created_by_user_id: Option<Uuid>,
    updated_by_user_id: Option<Uuid>,
    slug: String,
    image_url: Option<&str>,
    summary: Option<&str>,
    market_segment: Option<&str>,
    suggested_internal_tags: &[String],
    sources: &[String],
    featured: bool,
    visible: bool,
    searchable: bool,
) -> Result<(), AuthError> {
    let normalized_tags = normalize_catalog_tags(suggested_internal_tags);
    let normalized_sources = normalize_string_list(sources);

    crud::upsert_asset_catalog_entry(
        &state.db,
        asset_address,
        &slug,
        normalize_optional_text(image_url).as_deref(),
        normalize_optional_text(summary).as_deref(),
        normalize_optional_text(market_segment).as_deref(),
        &normalized_tags,
        &normalized_sources,
        featured,
        visible,
        searchable,
        created_by_user_id,
        updated_by_user_id,
    )
    .await?;

    Ok(())
}

async fn load_asset_record_for_admin_write(
    state: &AppState,
    asset_address: Address,
    actor_user_id: Uuid,
) -> Result<AssetRecord, AuthError> {
    match sync_asset(state, asset_address, None, Some(actor_user_id), None).await {
        Ok(record) => Ok(record),
        Err(error) => match crud::get_asset(
            &state.db,
            state.env.monad_chain_id,
            &format_address(asset_address),
        )
        .await?
        {
            Some(record) => Ok(record),
            None => Err(error),
        },
    }
}

async fn ensure_asset_creation_preconditions(
    state: &AppState,
    proposal_id: U256,
    asset_type_id: H256,
    catalog_slug: &str,
) -> Result<(), AuthError> {
    let existing_asset_address = read_factory_asset_address(&state.env, proposal_id).await?;
    if existing_asset_address != Address::zero() {
        let existing_asset_address = format_address(existing_asset_address);
        let state_suffix =
            crud::get_asset(&state.db, state.env.monad_chain_id, &existing_asset_address)
                .await?
                .map(|asset| format!(" current_state={}", asset.asset_state_label))
                .unwrap_or_default();

        return Err(AuthError::bad_request(format!(
            "proposal_id is already assigned to asset {}. Archived assets still reserve proposal IDs.{}",
            existing_asset_address, state_suffix
        )));
    }

    let (_, _, is_registered) = read_asset_type_from_chain(&state.env, asset_type_id).await?;
    if !is_registered {
        return Err(AuthError::bad_request(
            "asset_type_id is not currently registered in the asset factory",
        ));
    }

    if let Some(existing_asset) =
        crud::get_asset_by_slug(&state.db, state.env.monad_chain_id, catalog_slug).await?
    {
        return Err(AuthError::bad_request(format!(
            "asset slug `{}` is already in use by asset {}. Use a new slug when recreating an archived asset.",
            catalog_slug, existing_asset.asset_address
        )));
    }

    Ok(())
}

async fn optional_detail_section<T, F>(
    section: &'static str,
    future: F,
) -> (Option<T>, Option<String>)
where
    F: Future<Output = Result<T, AuthError>>,
{
    match future.await {
        Ok(value) => (Some(value), None),
        Err(error) => {
            tracing::warn!(section, %error, "asset detail section unavailable");
            (None, Some(section.to_owned()))
        }
    }
}

async fn record_asset_price_history(
    state: &AppState,
    asset: &AssetRecord,
    source: &str,
    actor_user_id: Option<Uuid>,
    tx_hash: Option<&str>,
) -> Result<(), AuthError> {
    crud::insert_asset_price_history(
        &state.db,
        &asset.asset_address,
        &asset.price_per_token,
        &asset.redemption_price_per_token,
        source,
        tx_hash,
        actor_user_id,
        Some(asset.updated_at),
    )
    .await
}

fn normalize_asset_history_range(raw: Option<&str>) -> Result<AssetHistoryWindow, AuthError> {
    let range = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_HISTORY_RANGE)
        .to_ascii_lowercase();
    let now = Utc::now();

    let window = match range.as_str() {
        "1day" => AssetHistoryWindow {
            range: "1day",
            interval_label: "5m",
            interval: Duration::minutes(5),
            observed_from: Some(now - Duration::days(1)),
        },
        "1week" => AssetHistoryWindow {
            range: "1week",
            interval_label: "1h",
            interval: Duration::hours(1),
            observed_from: Some(now - Duration::weeks(1)),
        },
        "1month" => AssetHistoryWindow {
            range: "1month",
            interval_label: "1d",
            interval: Duration::days(1),
            observed_from: Some(now - Duration::days(30)),
        },
        "3months" => AssetHistoryWindow {
            range: "3months",
            interval_label: "1d",
            interval: Duration::days(1),
            observed_from: Some(now - Duration::days(90)),
        },
        "1year" => AssetHistoryWindow {
            range: "1year",
            interval_label: "1w",
            interval: Duration::weeks(1),
            observed_from: Some(now - Duration::days(365)),
        },
        "all" => AssetHistoryWindow {
            range: "all",
            interval_label: "1w",
            interval: Duration::weeks(1),
            observed_from: None,
        },
        _ => {
            return Err(AuthError::bad_request(
                "range must be one of: 1day, 1week, 1month, 3months, 1year, all",
            ));
        }
    };

    Ok(window)
}

fn build_primary_history_samples(
    records: &[AssetPriceHistoryRecord],
) -> Result<Vec<HistorySample>, AuthError> {
    records
        .iter()
        .map(|record| {
            Ok(HistorySample {
                observed_at: record.observed_at,
                value: decimal_from_history_value(
                    &record.price_per_token,
                    "asset price history price_per_token",
                )?,
            })
        })
        .collect()
}

fn build_underlying_history_samples(
    records: &[OracleValuationHistoryRecord],
) -> Result<Vec<HistorySample>, AuthError> {
    records
        .iter()
        .map(|record| {
            Ok(HistorySample {
                observed_at: record.observed_at,
                value: decimal_from_history_value(
                    &record.nav_per_token,
                    "oracle valuation history nav_per_token",
                )?,
            })
        })
        .collect()
}

fn build_history_candles(
    samples: &[HistorySample],
    window: AssetHistoryWindow,
) -> Result<Vec<AssetHistoryCandleResponse>, AuthError> {
    if samples.is_empty() {
        return Ok(Vec::new());
    }

    let bucket_seconds = window.interval.num_seconds();
    if bucket_seconds <= 0 {
        return Err(AuthError::internal(
            "asset history interval must be positive",
            bucket_seconds,
        ));
    }

    let mut candles = Vec::new();
    let mut current_bucket_timestamp = None;
    let mut open = Decimal::ZERO;
    let mut high = Decimal::ZERO;
    let mut low = Decimal::ZERO;
    let mut close = Decimal::ZERO;

    for sample in samples {
        let bucket_timestamp = bucket_timestamp_millis(sample.observed_at, bucket_seconds)
            .map_err(|error| {
                AuthError::internal("failed to bucket asset history timestamp", error)
            })?;

        match current_bucket_timestamp {
            Some(timestamp) if timestamp == bucket_timestamp => {
                if sample.value > high {
                    high = sample.value;
                }
                if sample.value < low {
                    low = sample.value;
                }
                close = sample.value;
            }
            Some(timestamp) => {
                candles.push(history_candle_response(timestamp, open, high, low, close));
                current_bucket_timestamp = Some(bucket_timestamp);
                open = sample.value;
                high = sample.value;
                low = sample.value;
                close = sample.value;
            }
            None => {
                current_bucket_timestamp = Some(bucket_timestamp);
                open = sample.value;
                high = sample.value;
                low = sample.value;
                close = sample.value;
            }
        }
    }

    if let Some(timestamp) = current_bucket_timestamp {
        candles.push(history_candle_response(timestamp, open, high, low, close));
    }

    Ok(candles)
}

fn history_candle_response(
    timestamp: i64,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
) -> AssetHistoryCandleResponse {
    AssetHistoryCandleResponse {
        timestamp,
        value: close.normalize().to_string(),
        open: open.normalize().to_string(),
        high: high.normalize().to_string(),
        low: low.normalize().to_string(),
        close: close.normalize().to_string(),
    }
}

fn bucket_timestamp_millis(
    observed_at: DateTime<Utc>,
    bucket_seconds: i64,
) -> std::result::Result<i64, &'static str> {
    let timestamp = observed_at.timestamp();
    let bucket_start = timestamp - timestamp.rem_euclid(bucket_seconds);
    bucket_start
        .checked_mul(1000)
        .ok_or("asset history timestamp overflow")
}

fn decimal_from_history_value(raw: &str, context: &'static str) -> Result<Decimal, AuthError> {
    Decimal::from_str(raw).map_err(|error| AuthError::internal(context, error))
}

fn timestamp_seconds_to_utc(timestamp: i64) -> Result<DateTime<Utc>, AuthError> {
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .ok_or_else(|| AuthError::bad_request("history timestamp is out of range"))
}

fn normalize_list_assets_query(
    query: ListAssetsQuery,
) -> Result<NormalizedAssetListQuery, AuthError> {
    let asset_type_id = query
        .asset_type_id
        .as_deref()
        .map(|value| parse_bytes32_input(value, "asset_type_id").map(format_h256))
        .transpose()?;
    let q = normalize_optional_text(query.q.as_deref());
    let asset_state = query
        .asset_state
        .as_deref()
        .map(parse_asset_state)
        .transpose()?
        .map(i32::from);
    let limit = normalize_limit(query.limit)?;
    let offset = normalize_offset(query.offset)?;

    Ok(NormalizedAssetListQuery {
        asset_type_id,
        q,
        asset_state,
        self_service_purchase_enabled: query.self_service_purchase_enabled,
        featured: query.featured,
        limit,
        offset,
    })
}

fn build_catalog_slug(raw_slug: Option<&str>, fallback_name: &str) -> Result<String, AuthError> {
    match normalize_optional_text(raw_slug) {
        Some(value) => normalize_slug(&value, "asset slug"),
        None => normalize_slug(fallback_name, "asset name"),
    }
}

fn normalize_slug(raw: &str, field_name: &str) -> Result<String, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    let mut slug = String::with_capacity(trimmed.len());
    let mut previous_was_hyphen = false;

    for character in trimmed.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_was_hyphen = false;
            continue;
        }

        if !previous_was_hyphen {
            slug.push('-');
            previous_was_hyphen = true;
        }
    }

    let normalized = slug.trim_matches('-').to_owned();
    if normalized.is_empty() {
        return Err(AuthError::bad_request(format!(
            "{field_name} must contain letters or numbers",
        )));
    }

    Ok(normalized)
}

fn normalize_optional_text(raw: Option<&str>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

async fn ensure_factory_signer_has_any_role(
    env: &Environment,
    roles: &[(&'static str, H256)],
    action: &'static str,
) -> Result<(), AuthError> {
    let signer = admin_signer(env).await?;
    let signer_address = signer.address();
    let factory = read_factory_contract(env).await.map_err(|error| {
        AuthError::internal("failed to build asset factory read contract", error)
    })?;
    let access_control_address = factory
        .method::<_, Address>("accessControl", ())
        .map_err(|error| AuthError::internal("failed to build factory accessControl call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call factory accessControl", error))?;
    let access_control = read_access_control_contract(env, access_control_address).await?;

    for (_, role) in roles {
        let has_role = access_control
            .method::<_, bool>("hasRole", (*role, signer_address))
            .map_err(|error| {
                AuthError::internal("failed to build accessControl hasRole call", error)
            })?
            .call()
            .await
            .map_err(|error| AuthError::internal("failed to call accessControl hasRole", error))?;
        if has_role {
            return Ok(());
        }
    }

    let required_roles = roles
        .iter()
        .map(|(label, _)| *label)
        .collect::<Vec<_>>()
        .join(" or ");
    tracing::warn!(
        signer_address = %format_address(signer_address),
        access_control_address = %format_address(access_control_address),
        required_roles,
        action,
        "backend signer lacks required factory role"
    );

    Err(AuthError::forbidden(format!(
        "backend signer {} lacks {} on access control {} required to {}",
        format_address(signer_address),
        required_roles,
        format_address(access_control_address),
        action,
    )))
}

fn normalize_catalog_tags(raw: &[String]) -> Vec<String> {
    normalize_string_list_with(raw, |value| value.to_ascii_lowercase())
}

fn normalize_string_list(raw: &[String]) -> Vec<String> {
    normalize_string_list_with(raw, ToOwned::to_owned)
}

fn normalize_string_list_with<F>(raw: &[String], transform: F) -> Vec<String>
where
    F: Fn(&str) -> String,
{
    let mut normalized = Vec::new();

    for value in raw {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }

        let candidate = transform(trimmed);
        if !normalized.iter().any(|existing| existing == &candidate) {
            normalized.push(candidate);
        }
    }

    normalized
}

fn normalize_limit(raw: Option<i64>) -> Result<i64, AuthError> {
    let value = raw.unwrap_or(DEFAULT_LIST_LIMIT);
    if !(1..=MAX_LIST_LIMIT).contains(&value) {
        return Err(AuthError::bad_request(format!(
            "limit must be between 1 and {MAX_LIST_LIMIT}",
        )));
    }

    Ok(value)
}

fn normalize_offset(raw: Option<i64>) -> Result<i64, AuthError> {
    let value = raw.unwrap_or(0);
    if value < 0 {
        return Err(AuthError::bad_request(
            "offset must be greater than or equal to zero",
        ));
    }

    Ok(value)
}

async fn read_factory_contract(env: &Environment) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.asset_factory_address)?,
        asset_factory_abi()?,
        provider,
    ))
}

async fn read_access_control_contract(
    env: &Environment,
    access_control_address: Address,
) -> Result<Contract<Provider<Http>>, AuthError> {
    let provider = rpc::monad_provider_arc(env)
        .await
        .map_err(|error| AuthError::internal("failed to build access control provider", error))?;
    let abi = AbiParser::default()
        .parse(&["function hasRole(bytes32 role, address account) view returns (bool)"])
        .map_err(|error| AuthError::internal("failed to build access control ABI", error))?;

    Ok(Contract::new(access_control_address, abi, provider))
}

async fn write_factory_contract(
    env: &Environment,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.asset_factory_address)
            .map_err(|error| AuthError::internal("invalid asset factory address", error))?,
        asset_factory_abi()
            .map_err(|error| AuthError::internal("failed to build asset factory ABI", error))?,
        signer,
    ))
}

async fn read_asset_contract(
    env: &Environment,
    asset_address: Address,
) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(
        asset_address,
        base_asset_token_abi()?,
        provider,
    ))
}

async fn write_asset_contract(
    env: &Environment,
    asset_address: Address,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        asset_address,
        base_asset_token_abi()
            .map_err(|error| AuthError::internal("failed to build asset token ABI", error))?,
        signer,
    ))
}

async fn read_erc20_contract(
    env: &Environment,
    token_address: Address,
) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(token_address, erc20_abi()?, provider))
}

async fn send_factory_transaction<T, D>(
    env: &Environment,
    method: &str,
    args: T,
    error_context: &'static str,
) -> Result<String, AuthError>
where
    T: Tokenize,
    D: Detokenize,
{
    let contract = write_factory_contract(env).await?;
    let call = contract
        .method::<_, D>(method, args)
        .map_err(|error| AuthError::internal("failed to build asset factory transaction", error))?;
    let pending = call
        .send()
        .await
        .map_err(|error| AuthError::internal(error_context, error))?;

    wait_for_receipt(pending).await
}

fn role_hash(value: &str) -> H256 {
    H256::from(keccak256(value.as_bytes()))
}

async fn send_asset_transaction<T, D>(
    env: &Environment,
    asset_address: Address,
    method: &str,
    args: T,
    error_context: &'static str,
) -> Result<String, AuthError>
where
    T: Tokenize,
    D: Detokenize,
{
    let contract = write_asset_contract(env, asset_address).await?;
    let call = contract
        .method::<_, D>(method, args)
        .map_err(|error| AuthError::internal("failed to build asset transaction", error))?;
    let pending = call
        .send()
        .await
        .map_err(|error| AuthError::internal(error_context, error))?;

    wait_for_receipt(pending).await
}

async fn build_asset_calldata<T, D>(
    env: &Environment,
    asset_address: Address,
    method: &str,
    args: T,
) -> Result<Bytes, AuthError>
where
    T: Tokenize,
    D: Detokenize,
{
    let contract = read_asset_contract(env, asset_address)
        .await
        .map_err(|error| AuthError::internal("failed to build asset read contract", error))?;
    contract
        .method::<_, D>(method, args)
        .map_err(|error| AuthError::internal("failed to build asset calldata", error))?
        .calldata()
        .ok_or_else(|| AuthError::internal("missing asset calldata", anyhow!("no calldata")))
}

async fn build_erc20_calldata<T, D>(
    env: &Environment,
    token_address: Address,
    method: &str,
    args: T,
) -> Result<Bytes, AuthError>
where
    T: Tokenize,
    D: Detokenize,
{
    let contract = read_erc20_contract(env, token_address)
        .await
        .map_err(|error| AuthError::internal("failed to build ERC20 read contract", error))?;
    contract
        .method::<_, D>(method, args)
        .map_err(|error| AuthError::internal("failed to build ERC20 calldata", error))?
        .calldata()
        .ok_or_else(|| AuthError::internal("missing ERC20 calldata", anyhow!("no calldata")))
}

pub async fn list_pending_redemptions(
    state: &AppState,
    asset_address: &str,
) -> Result<AssetPendingRedemptionsResponse, AuthError> {
    let asset_address_parsed = parse_address(asset_address)?;
    let asset_address_string = format_address(asset_address_parsed);

    // Get asset info
    let asset_record = crud::get_asset(&state.db, state.env.monad_chain_id, &asset_address_string)
        .await?
        .ok_or_else(|| AuthError::not_found("asset not found"))?;

    // Get ALL wallet accounts (users who might have this asset)
    let all_wallets = sqlx::query_as::<_, WalletAccountLookup>(
        "SELECT DISTINCT wa.user_id, wa.wallet_address, u.email, u.display_name
         FROM wallet_accounts wa
         JOIN users u ON wa.user_id = u.id
         ORDER BY wa.wallet_address ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| AuthError::internal("failed to fetch wallet accounts", error))?;

    // Query blockchain for each wallet's pending redemption
    let mut pending_redemptions = Vec::new();

    for wallet_record in all_wallets {
        let wallet_address = match parse_address(&wallet_record.wallet_address) {
            Ok(addr) => addr,
            Err(_) => continue, // Skip invalid addresses
        };

        // Query blockchain for actual pending amount
        match read_asset_holder_snapshot_from_chain(
            &state.env,
            asset_address_parsed,
            wallet_address,
        )
        .await
        {
            Ok(holder) => {
                let pending_amount = match U256::from_dec_str(&holder.pending_redemption) {
                    Ok(amount) => amount,
                    Err(_) => continue,
                };

                // Only include if they actually have pending redemptions
                if pending_amount > U256::zero() {
                    pending_redemptions.push(PendingRedemptionItem {
                        user_id: wallet_record.user_id.to_string(),
                        wallet_address: wallet_record.wallet_address,
                        email: wallet_record.email,
                        display_name: wallet_record.display_name,
                        pending_amount: pending_amount.to_string(),
                        last_redemption_at: Utc::now(), // We don't have this info without trade history
                    });
                }
            }
            Err(_) => continue, // Skip wallets that don't hold this asset
        }
    }

    Ok(AssetPendingRedemptionsResponse {
        asset_address: asset_address_string,
        asset_name: asset_record.name,
        asset_symbol: asset_record.symbol,
        asset_image_url: asset_record.image_url,
        total_pending_redemptions: asset_record.total_pending_redemptions,
        pending_redemptions,
    })
}
