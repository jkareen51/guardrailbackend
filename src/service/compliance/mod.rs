use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use ethers_contract::Contract;
use ethers_core::types::{Address, H256, U256};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Signer};
use uuid::Uuid;

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        auth::error::AuthError,
        compliance::{
            crud,
            schema::{
                AdminBatchUpsertComplianceInvestorsRequest,
                AdminComplianceAssetRulesUpsertResponse,
                AdminComplianceInvestorBatchUpsertResponse, AdminComplianceInvestorUpsertResponse,
                AdminComplianceJurisdictionRestrictionUpsertResponse,
                AdminSetComplianceAssetRulesRequest,
                AdminSetComplianceJurisdictionRestrictionRequest,
                AdminUpsertComplianceInvestorRequest, ComplianceAssetRulesResponse,
                ComplianceCheckRedeemRequest, ComplianceCheckResponse,
                ComplianceCheckSubscribeRequest, ComplianceCheckTransferRequest,
                ComplianceInvestorResponse, ComplianceJurisdictionRestrictionResponse,
            },
        },
    },
    service::rpc,
};

use self::abi::compliance_registry_abi;

pub mod abi;

type InvestorDataTuple = (bool, bool, bool, u64, H256, H256);
type AssetRulesTuple = (bool, bool, bool, bool, U256, U256);

pub async fn upsert_investor(
    state: &AppState,
    actor_user_id: Uuid,
    wallet_address: &str,
    payload: AdminUpsertComplianceInvestorRequest,
) -> Result<AdminComplianceInvestorUpsertResponse, AuthError> {
    let wallet = parse_address(wallet_address)?;
    let investor = build_investor_tuple(
        payload.is_verified,
        payload.is_accredited,
        payload.is_frozen,
        payload.valid_until,
        &payload.jurisdiction,
        payload.external_ref.as_deref(),
    )?;

    let tx_hash = write_set_investor_data(&state.env, wallet, investor).await?;
    let record = crud::upsert_investor(
        &state.db,
        &format_address(wallet),
        payload.is_verified,
        payload.is_accredited,
        payload.is_frozen,
        valid_until_to_i64(payload.valid_until)?,
        &format_h256(investor.4),
        &format_h256(investor.5),
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AdminComplianceInvestorUpsertResponse {
        tx_hash,
        investor: ComplianceInvestorResponse::from_record(record),
    })
}

pub async fn batch_upsert_investors(
    state: &AppState,
    actor_user_id: Uuid,
    payload: AdminBatchUpsertComplianceInvestorsRequest,
) -> Result<AdminComplianceInvestorBatchUpsertResponse, AuthError> {
    if payload.investors.is_empty() {
        return Err(AuthError::bad_request("investors batch cannot be empty"));
    }

    let mut addresses = Vec::with_capacity(payload.investors.len());
    let mut tuples = Vec::with_capacity(payload.investors.len());
    let mut records = Vec::with_capacity(payload.investors.len());

    for item in &payload.investors {
        let address = parse_address(&item.wallet_address)?;
        let investor = build_investor_tuple(
            item.is_verified,
            item.is_accredited,
            item.is_frozen,
            item.valid_until,
            &item.jurisdiction,
            item.external_ref.as_deref(),
        )?;

        addresses.push(address);
        tuples.push(investor);
        records.push((
            format_address(address),
            item.is_verified,
            item.is_accredited,
            item.is_frozen,
            valid_until_to_i64(item.valid_until)?,
            format_h256(investor.4),
            format_h256(investor.5),
        ));
    }

    let tx_hash = write_batch_set_investor_data(&state.env, addresses, tuples).await?;

    let mut response_investors = Vec::with_capacity(records.len());
    for record in records {
        let db_record = crud::upsert_investor(
            &state.db,
            &record.0,
            record.1,
            record.2,
            record.3,
            record.4,
            &record.5,
            &record.6,
            Some(actor_user_id),
            Some(&tx_hash),
        )
        .await?;
        response_investors.push(ComplianceInvestorResponse::from_record(db_record));
    }

    Ok(AdminComplianceInvestorBatchUpsertResponse {
        tx_hash,
        investors: response_investors,
    })
}

pub async fn set_asset_rules(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    payload: AdminSetComplianceAssetRulesRequest,
) -> Result<AdminComplianceAssetRulesUpsertResponse, AuthError> {
    let asset = parse_address(asset_address)?;
    let asset_rules = build_asset_rules_tuple(
        payload.transfers_enabled,
        payload.subscriptions_enabled,
        payload.redemptions_enabled,
        payload.requires_accreditation,
        &payload.min_investment,
        &payload.max_investor_balance,
    )?;

    let tx_hash = write_set_asset_rules(&state.env, asset, asset_rules).await?;
    let record = crud::upsert_asset_rules(
        &state.db,
        &format_address(asset),
        payload.transfers_enabled,
        payload.subscriptions_enabled,
        payload.redemptions_enabled,
        payload.requires_accreditation,
        &u256_to_string(asset_rules.4),
        &u256_to_string(asset_rules.5),
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AdminComplianceAssetRulesUpsertResponse {
        tx_hash,
        asset_rules: ComplianceAssetRulesResponse::from(record),
    })
}

pub async fn set_jurisdiction_restriction(
    state: &AppState,
    actor_user_id: Uuid,
    asset_address: &str,
    jurisdiction: &str,
    payload: AdminSetComplianceJurisdictionRestrictionRequest,
) -> Result<AdminComplianceJurisdictionRestrictionUpsertResponse, AuthError> {
    let asset = parse_address(asset_address)?;
    let jurisdiction = parse_bytes32_input(jurisdiction, "jurisdiction")?;

    let tx_hash =
        write_set_jurisdiction_restriction(&state.env, asset, jurisdiction, payload.restricted)
            .await?;
    let record = crud::upsert_jurisdiction_restriction(
        &state.db,
        &format_address(asset),
        &format_h256(jurisdiction),
        payload.restricted,
        Some(actor_user_id),
        Some(&tx_hash),
    )
    .await?;

    Ok(AdminComplianceJurisdictionRestrictionUpsertResponse {
        tx_hash,
        restriction: ComplianceJurisdictionRestrictionResponse::from(record),
    })
}

pub async fn get_investor(
    state: &AppState,
    wallet_address: &str,
) -> Result<ComplianceInvestorResponse, AuthError> {
    let wallet = parse_address(wallet_address)?;
    let wallet_address = format_address(wallet);

    if let Some(record) = crud::get_investor(&state.db, &wallet_address).await? {
        return Ok(ComplianceInvestorResponse::from_record(record));
    }

    let (is_verified, is_accredited, is_frozen, valid_until, jurisdiction, external_ref) =
        read_investor_from_chain(&state.env, wallet)
            .await
            .map_err(|error| AuthError::internal("failed to read investor from chain", error))?;
    let record = crud::upsert_investor(
        &state.db,
        &wallet_address,
        is_verified,
        is_accredited,
        is_frozen,
        u64_to_i64(valid_until, "valid_until")?,
        &format_h256(jurisdiction),
        &format_h256(external_ref),
        None,
        None,
    )
    .await?;

    Ok(ComplianceInvestorResponse::from_record(record))
}

pub async fn get_asset_rules(
    state: &AppState,
    asset_address: &str,
) -> Result<ComplianceAssetRulesResponse, AuthError> {
    let asset = parse_address(asset_address)?;
    let asset_address = format_address(asset);

    if let Some(record) = crud::get_asset_rules(&state.db, &asset_address).await? {
        return Ok(ComplianceAssetRulesResponse::from(record));
    }

    let (
        transfers_enabled,
        subscriptions_enabled,
        redemptions_enabled,
        requires_accreditation,
        min_investment,
        max_investor_balance,
    ) = read_asset_rules_from_chain(&state.env, asset)
        .await
        .map_err(|error| AuthError::internal("failed to read asset rules from chain", error))?;
    let record = crud::upsert_asset_rules(
        &state.db,
        &asset_address,
        transfers_enabled,
        subscriptions_enabled,
        redemptions_enabled,
        requires_accreditation,
        &u256_to_string(min_investment),
        &u256_to_string(max_investor_balance),
        None,
        None,
    )
    .await?;

    Ok(ComplianceAssetRulesResponse::from(record))
}

pub async fn get_jurisdiction_restriction(
    state: &AppState,
    asset_address: &str,
    jurisdiction: &str,
) -> Result<ComplianceJurisdictionRestrictionResponse, AuthError> {
    let asset = parse_address(asset_address)?;
    let jurisdiction = parse_bytes32_input(jurisdiction, "jurisdiction")?;
    let asset_address = format_address(asset);
    let jurisdiction_hex = format_h256(jurisdiction);

    if let Some(record) =
        crud::get_jurisdiction_restriction(&state.db, &asset_address, &jurisdiction_hex).await?
    {
        return Ok(ComplianceJurisdictionRestrictionResponse::from(record));
    }

    let restricted = read_jurisdiction_restriction_from_chain(&state.env, asset, jurisdiction)
        .await
        .map_err(|error| {
            AuthError::internal("failed to read jurisdiction restriction from chain", error)
        })?;
    let record = crud::upsert_jurisdiction_restriction(
        &state.db,
        &asset_address,
        &jurisdiction_hex,
        restricted,
        None,
        None,
    )
    .await?;

    Ok(ComplianceJurisdictionRestrictionResponse::from(record))
}

pub async fn check_subscribe(
    state: &AppState,
    payload: ComplianceCheckSubscribeRequest,
) -> Result<ComplianceCheckResponse, AuthError> {
    let result = read_check_subscribe(
        &state.env,
        parse_address(&payload.asset_address)?,
        parse_address(&payload.investor_wallet)?,
        parse_u256(&payload.amount, "amount")?,
        parse_u256(&payload.resulting_balance, "resulting_balance")?,
    )
    .await?;

    Ok(ComplianceCheckResponse {
        is_valid: result.0,
        reason: bytes32_reason(result.1),
    })
}

pub async fn check_transfer(
    state: &AppState,
    payload: ComplianceCheckTransferRequest,
) -> Result<ComplianceCheckResponse, AuthError> {
    let result = read_check_transfer(
        &state.env,
        parse_address(&payload.asset_address)?,
        parse_address(&payload.from_wallet)?,
        parse_address(&payload.to_wallet)?,
        parse_u256(&payload.amount, "amount")?,
        parse_u256(&payload.receiving_balance, "receiving_balance")?,
    )
    .await?;

    Ok(ComplianceCheckResponse {
        is_valid: result.0,
        reason: bytes32_reason(result.1),
    })
}

pub async fn check_redeem(
    state: &AppState,
    payload: ComplianceCheckRedeemRequest,
) -> Result<ComplianceCheckResponse, AuthError> {
    let result = read_check_redeem(
        &state.env,
        parse_address(&payload.asset_address)?,
        parse_address(&payload.investor_wallet)?,
        parse_u256(&payload.amount, "amount")?,
    )
    .await?;

    Ok(ComplianceCheckResponse {
        is_valid: result.0,
        reason: bytes32_reason(result.1),
    })
}

fn build_investor_tuple(
    is_verified: bool,
    is_accredited: bool,
    is_frozen: bool,
    valid_until: Option<i64>,
    jurisdiction: &str,
    external_ref: Option<&str>,
) -> Result<InvestorDataTuple, AuthError> {
    Ok((
        is_verified,
        is_accredited,
        is_frozen,
        valid_until_to_u64(valid_until)?,
        parse_bytes32_input(jurisdiction, "jurisdiction")?,
        parse_bytes32_input(external_ref.unwrap_or_default(), "external_ref")?,
    ))
}

fn build_asset_rules_tuple(
    transfers_enabled: bool,
    subscriptions_enabled: bool,
    redemptions_enabled: bool,
    requires_accreditation: bool,
    min_investment: &str,
    max_investor_balance: &str,
) -> Result<AssetRulesTuple, AuthError> {
    Ok((
        transfers_enabled,
        subscriptions_enabled,
        redemptions_enabled,
        requires_accreditation,
        parse_u256(min_investment, "min_investment")?,
        parse_u256(max_investor_balance, "max_investor_balance")?,
    ))
}

async fn write_set_investor_data(
    env: &Environment,
    wallet: Address,
    investor: InvestorDataTuple,
) -> Result<String, AuthError> {
    let contract = write_contract(env).await?;
    let call = contract
        .method::<_, ()>("setInvestorData", (wallet, investor))
        .map_err(|error| AuthError::internal("failed to build setInvestorData call", error))?;
    let pending = call.send().await.map_err(|error| {
        AuthError::internal("failed to submit setInvestorData transaction", error)
    })?;

    wait_for_receipt(pending).await
}

async fn write_batch_set_investor_data(
    env: &Environment,
    wallets: Vec<Address>,
    investors: Vec<InvestorDataTuple>,
) -> Result<String, AuthError> {
    let contract = write_contract(env).await?;
    let call = contract
        .method::<_, ()>("batchSetInvestorData", (wallets, investors))
        .map_err(|error| AuthError::internal("failed to build batchSetInvestorData call", error))?;
    let pending = call.send().await.map_err(|error| {
        AuthError::internal("failed to submit batchSetInvestorData transaction", error)
    })?;

    wait_for_receipt(pending).await
}

async fn write_set_asset_rules(
    env: &Environment,
    asset: Address,
    rules: AssetRulesTuple,
) -> Result<String, AuthError> {
    let contract = write_contract(env).await?;
    let call = contract
        .method::<_, ()>("setAssetRules", (asset, rules))
        .map_err(|error| AuthError::internal("failed to build setAssetRules call", error))?;
    let pending = call.send().await.map_err(|error| {
        AuthError::internal("failed to submit setAssetRules transaction", error)
    })?;

    wait_for_receipt(pending).await
}

async fn write_set_jurisdiction_restriction(
    env: &Environment,
    asset: Address,
    jurisdiction: H256,
    restricted: bool,
) -> Result<String, AuthError> {
    let contract = write_contract(env).await?;
    let call = contract
        .method::<_, ()>(
            "setJurisdictionRestriction",
            (asset, jurisdiction, restricted),
        )
        .map_err(|error| {
            AuthError::internal("failed to build setJurisdictionRestriction call", error)
        })?;
    let pending = call.send().await.map_err(|error| {
        AuthError::internal(
            "failed to submit setJurisdictionRestriction transaction",
            error,
        )
    })?;

    wait_for_receipt(pending).await
}

async fn read_investor_from_chain(env: &Environment, wallet: Address) -> Result<InvestorDataTuple> {
    let contract = read_contract(env).await?;
    contract
        .method::<_, InvestorDataTuple>("getInvestorData", wallet)?
        .call()
        .await
        .context("failed to call getInvestorData")
}

async fn read_asset_rules_from_chain(env: &Environment, asset: Address) -> Result<AssetRulesTuple> {
    let contract = read_contract(env).await?;
    contract
        .method::<_, AssetRulesTuple>("getAssetRules", asset)?
        .call()
        .await
        .context("failed to call getAssetRules")
}

async fn read_jurisdiction_restriction_from_chain(
    env: &Environment,
    asset: Address,
    jurisdiction: H256,
) -> Result<bool> {
    let contract = read_contract(env).await?;
    contract
        .method::<_, bool>("isJurisdictionRestricted", (asset, jurisdiction))?
        .call()
        .await
        .context("failed to call isJurisdictionRestricted")
}

async fn read_check_subscribe(
    env: &Environment,
    asset: Address,
    investor: Address,
    amount: U256,
    resulting_balance: U256,
) -> Result<(bool, H256), AuthError> {
    let contract = read_contract(env)
        .await
        .map_err(|error| AuthError::internal("failed to build compliance read contract", error))?;
    contract
        .method::<_, (bool, H256)>("canSubscribe", (asset, investor, amount, resulting_balance))
        .map_err(|error| AuthError::internal("failed to build canSubscribe call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call canSubscribe", error))
}

async fn read_check_transfer(
    env: &Environment,
    asset: Address,
    from: Address,
    to: Address,
    amount: U256,
    receiving_balance: U256,
) -> Result<(bool, H256), AuthError> {
    let contract = read_contract(env)
        .await
        .map_err(|error| AuthError::internal("failed to build compliance read contract", error))?;
    contract
        .method::<_, (bool, H256)>("canTransfer", (asset, from, to, amount, receiving_balance))
        .map_err(|error| AuthError::internal("failed to build canTransfer call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call canTransfer", error))
}

async fn read_check_redeem(
    env: &Environment,
    asset: Address,
    investor: Address,
    amount: U256,
) -> Result<(bool, H256), AuthError> {
    let contract = read_contract(env)
        .await
        .map_err(|error| AuthError::internal("failed to build compliance read contract", error))?;
    contract
        .method::<_, (bool, H256)>("canRedeem", (asset, investor, amount))
        .map_err(|error| AuthError::internal("failed to build canRedeem call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call canRedeem", error))
}

async fn read_contract(env: &Environment) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.compliance_registry_address)?,
        compliance_registry_abi()?,
        provider,
    ))
}

async fn write_contract(
    env: &Environment,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.compliance_registry_address)
            .map_err(|error| AuthError::internal("invalid compliance registry address", error))?,
        compliance_registry_abi().map_err(|error| {
            AuthError::internal("failed to build compliance registry ABI", error)
        })?,
        signer,
    ))
}

async fn admin_signer(
    env: &Environment,
) -> Result<std::sync::Arc<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
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

async fn wait_for_receipt<P>(
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

fn parse_address(raw: &str) -> Result<Address, AuthError> {
    raw.trim()
        .parse::<Address>()
        .map_err(|_| AuthError::bad_request("invalid address"))
}

fn parse_contract_address(raw: &str) -> Result<Address> {
    raw.trim()
        .parse::<Address>()
        .with_context(|| format!("invalid contract address `{raw}`"))
}

fn format_address(address: Address) -> String {
    format!("{address:#x}")
}

fn parse_bytes32_input(raw: &str, field_name: &str) -> Result<H256, AuthError> {
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

fn parse_u256(raw: &str, field_name: &str) -> Result<U256, AuthError> {
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

fn valid_until_to_u64(valid_until: Option<i64>) -> Result<u64, AuthError> {
    match valid_until {
        Some(value) if value < 0 => Err(AuthError::bad_request("valid_until cannot be negative")),
        Some(value) => {
            u64::try_from(value).map_err(|_| AuthError::bad_request("valid_until is out of range"))
        }
        None => Ok(0),
    }
}

fn valid_until_to_i64(valid_until: Option<i64>) -> Result<i64, AuthError> {
    match valid_until {
        Some(value) if value < 0 => Err(AuthError::bad_request("valid_until cannot be negative")),
        Some(value) => Ok(value),
        None => Ok(0),
    }
}

fn u64_to_i64(value: u64, field_name: &str) -> Result<i64, AuthError> {
    i64::try_from(value)
        .map_err(|_| AuthError::bad_request(format!("{field_name} is out of range")))
}

fn format_h256(value: H256) -> String {
    format!("{value:#x}")
}

fn u256_to_string(value: U256) -> String {
    value.to_string()
}

fn bytes32_reason(value: H256) -> String {
    let bytes = value.as_bytes();
    if bytes.iter().all(|byte| *byte == 0) {
        return "UNKNOWN".to_owned();
    }

    let trimmed = bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .collect::<Vec<_>>();

    if !trimmed.is_empty()
        && bytes[trimmed.len()..].iter().all(|byte| *byte == 0)
        && trimmed
            .iter()
            .all(|value| value.is_ascii_graphic() || *value == b'_')
    {
        if let Ok(text) = String::from_utf8(trimmed) {
            return text;
        }
    }

    format_h256(value)
}

#[cfg(test)]
mod tests {
    use super::{bytes32_reason, format_h256, parse_bytes32_input};
    use ethers_core::types::H256;

    #[test]
    fn parses_plain_text_bytes32() {
        let value = parse_bytes32_input("NG", "jurisdiction").expect("bytes32");
        assert_eq!(
            format_h256(value),
            "0x4e47000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn preserves_hex_reason_when_not_ascii() {
        let value = H256::from_low_u64_be(42);
        assert_eq!(bytes32_reason(value), format_h256(value));
    }
}
