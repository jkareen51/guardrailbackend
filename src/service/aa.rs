use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use ethers_contract::Contract;
use ethers_core::{
    abi::{Token, encode},
    types::{Address, Bytes, H256, U256},
    utils::keccak256,
};
use ethers_providers::{Http, Middleware, Provider};
use ethers_signers::{LocalWallet, Signer};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use uuid::Uuid;

use crate::{
    config::environment::Environment,
    module::auth::model::{NewWalletRecord, OWNER_PROVIDER_LOCAL},
    service::{
        crypto::{create_managed_owner_key, decrypt_private_key},
        rpc,
    },
};

use super::liquidity::abi::{entry_point_abi, simple_account_abi, simple_account_factory_abi};

#[derive(Debug, Clone)]
pub struct SmartAccountSignerContext {
    pub wallet_address: String,
    pub owner_address: String,
    pub owner_provider: String,
    pub owner_ref: String,
    pub factory_address: String,
    pub entry_point_address: String,
    pub owner_encrypted_private_key: String,
    pub owner_encryption_nonce: String,
}

#[derive(Debug, Clone)]
pub struct SmartAccountCall {
    pub target: Address,
    pub data: Bytes,
}

#[derive(Debug, Clone)]
pub struct SmartAccountExecutionResult {
    pub tx_hash: String,
}

pub async fn provision_local_smart_account(
    env: &Environment,
    user_id: Uuid,
) -> Result<NewWalletRecord> {
    let owner = create_managed_owner_key(env)?;
    let owner_address = parse_address(&owner.owner_address)?;
    let smart_account_address =
        derive_smart_account_address(env, owner_address, user_operation_salt(user_id)).await?;

    Ok(NewWalletRecord::smart_account(
        format!("{smart_account_address:#x}"),
        env.monad_chain_id,
        owner.owner_address.clone(),
        OWNER_PROVIDER_LOCAL.to_owned(),
        format!("{:#x}", user_operation_salt(user_id)),
        env.aa_simple_account_factory_address.clone(),
        env.aa_entry_point_address.clone(),
        owner.encrypted_private_key,
        owner.encryption_nonce,
        owner.key_version,
    ))
}

pub async fn submit_calls(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    calls: &[SmartAccountCall],
) -> Result<SmartAccountExecutionResult> {
    if calls.is_empty() {
        return Err(anyhow!(
            "smart-account execution requires at least one call"
        ));
    }

    let provider = chain_provider(env).await?;
    let sender = parse_address(&signer.wallet_address)?;
    let owner_address = parse_address(&signer.owner_address)?;
    let factory_address = parse_address(&signer.factory_address)?;
    let entry_point_address = parse_address(&signer.entry_point_address)?;
    let salt = parse_u256_hex(&signer.owner_ref)
        .with_context(|| format!("invalid smart-account salt `{}`", signer.owner_ref))?;
    let init_code =
        build_init_code(&provider, factory_address, owner_address, salt, sender).await?;
    let nonce = load_nonce(&provider, entry_point_address, sender, init_code.is_empty()).await?;
    let call_data = build_account_call_data(&provider, sender, calls)?;
    let gas_price = load_user_operation_gas_price(env, http_client).await?;
    let mut user_op = UserOperationData {
        sender,
        nonce,
        init_code,
        call_data,
        call_gas_limit: U256::zero(),
        verification_gas_limit: U256::zero(),
        pre_verification_gas: U256::zero(),
        max_fee_per_gas: gas_price.max_fee_per_gas,
        max_priority_fee_per_gas: gas_price.max_priority_fee_per_gas,
        paymaster_and_data: Bytes::default(),
        signature: dummy_user_operation_signature().to_owned(),
    };

    let variant =
        sponsor_user_operation(env, http_client, &mut user_op, &signer.entry_point_address).await?;

    let owner_private_key = decrypt_private_key(
        env,
        &signer.owner_encrypted_private_key,
        &signer.owner_encryption_nonce,
    )
    .context("failed to decrypt smart-account owner key")?;
    let wallet = format!("0x{}", hex::encode(owner_private_key))
        .parse::<LocalWallet>()
        .context("invalid decrypted smart-account owner key")?
        .with_chain_id(env.monad_chain_id as u64);
    let user_op_hash = user_operation_hash(
        &provider,
        &user_op,
        entry_point_address,
        U256::from(env.monad_chain_id as u64),
        variant,
    )
    .await?;
    let signature = match variant {
        UserOperationVariant::LegacyV06 => wallet
            .sign_message(user_op_hash)
            .await
            .context("failed to sign legacy user operation")?,
        UserOperationVariant::PackedV07 => wallet
            .sign_hash(H256::from(user_op_hash))
            .context("failed to sign packed user operation")?,
    };
    user_op.signature = signature.to_string();

    let user_op_hash = send_user_operation(
        env,
        http_client,
        &user_op,
        &signer.entry_point_address,
        variant,
    )
    .await?;
    let tx_hash = wait_for_user_operation_receipt(env, http_client, &user_op_hash).await?;

    Ok(SmartAccountExecutionResult { tx_hash })
}

pub fn user_operation_salt(user_id: Uuid) -> U256 {
    U256::from_big_endian(user_id.as_bytes())
}

async fn derive_smart_account_address(
    env: &Environment,
    owner_address: Address,
    salt: U256,
) -> Result<Address> {
    let provider = chain_provider(env).await?;
    let factory = Contract::new(
        parse_address(&env.aa_simple_account_factory_address)?,
        simple_account_factory_abi()?,
        provider,
    );

    factory
        .method::<_, Address>("getAddress", (owner_address, salt))?
        .call()
        .await
        .context("failed to derive smart-account address")
}

async fn build_init_code(
    provider: &Arc<Provider<Http>>,
    factory_address: Address,
    owner_address: Address,
    salt: U256,
    sender: Address,
) -> Result<Bytes> {
    let code = provider
        .get_code(sender, None)
        .await
        .context("failed to fetch smart-account code")?;
    if !code.as_ref().is_empty() {
        return Ok(Bytes::default());
    }

    let factory = Contract::new(
        factory_address,
        simple_account_factory_abi()?,
        provider.clone(),
    );
    let call_data = factory
        .method::<_, Address>("createAccount", (owner_address, salt))?
        .calldata()
        .ok_or_else(|| anyhow!("missing smart-account createAccount calldata"))?;

    let mut init_code = factory_address.as_bytes().to_vec();
    init_code.extend_from_slice(call_data.as_ref());
    Ok(Bytes::from(init_code))
}

async fn load_nonce(
    provider: &Arc<Provider<Http>>,
    entry_point_address: Address,
    sender: Address,
    deployed: bool,
) -> Result<U256> {
    if !deployed {
        return Ok(U256::zero());
    }

    let entry_point = Contract::new(entry_point_address, entry_point_abi()?, provider.clone());
    entry_point
        .method::<_, U256>("getNonce", (sender, 0_u64))?
        .call()
        .await
        .context("failed to load smart-account nonce")
}

fn build_account_call_data(
    provider: &Arc<Provider<Http>>,
    sender: Address,
    calls: &[SmartAccountCall],
) -> Result<Bytes> {
    let account = Contract::new(sender, simple_account_abi()?, provider.clone());

    if calls.len() == 1 {
        return account
            .method::<_, ()>(
                "execute",
                (calls[0].target, U256::zero(), calls[0].data.clone()),
            )?
            .calldata()
            .ok_or_else(|| anyhow!("missing smart-account execute calldata"));
    }

    let targets = calls.iter().map(|call| call.target).collect::<Vec<_>>();
    let data = calls
        .iter()
        .map(|call| call.data.clone())
        .collect::<Vec<_>>();

    account
        .method::<_, ()>("executeBatch", (targets, data))?
        .calldata()
        .ok_or_else(|| anyhow!("missing smart-account executeBatch calldata"))
}

async fn sponsor_user_operation(
    env: &Environment,
    http_client: &Client,
    user_op: &mut UserOperationData,
    entry_point_address: &str,
) -> Result<UserOperationVariant> {
    let variants = preferred_user_operation_variants(entry_point_address);
    let mut last_error = None;

    for variant in variants {
        match sponsor_user_operation_with_variant(
            env,
            http_client,
            user_op,
            entry_point_address,
            variant,
        )
        .await
        {
            Ok(()) => return Ok(variant),
            Err(error) => {
                if !is_user_operation_schema_mismatch(&error) {
                    return Err(error.context(format!(
                        "user-operation sponsorship failed using {} format",
                        variant.label()
                    )));
                }

                last_error = Some((variant, error));
            }
        }
    }

    let (last_variant, last_error) = last_error
        .ok_or_else(|| anyhow!("no user-operation variants were attempted for sponsorship"))?;
    Err(anyhow!(
        "failed to sponsor user operation using {} format: {last_error}",
        last_variant.label()
    ))
}

async fn sponsor_user_operation_with_variant(
    env: &Environment,
    http_client: &Client,
    user_op: &mut UserOperationData,
    entry_point_address: &str,
    variant: UserOperationVariant,
) -> Result<()> {
    match variant {
        UserOperationVariant::LegacyV06 => {
            let response: JsonRpcResponse<LegacySponsoredUserOperation> = post_json_rpc(
                http_client,
                &env.aa_bundler_rpc_url,
                "pm_sponsorUserOperation",
                serde_json::json!([
                    LegacyUserOperationRequest::from(&*user_op),
                    entry_point_address
                ]),
            )
            .await?;
            let sponsored = response
                .result
                .ok_or_else(|| anyhow!("paymaster did not return sponsorship data"))?;
            apply_legacy_sponsorship(user_op, &sponsored)
        }
        UserOperationVariant::PackedV07 => {
            let response: JsonRpcResponse<ModernSponsoredUserOperation> = post_json_rpc(
                http_client,
                &env.aa_bundler_rpc_url,
                "pm_sponsorUserOperation",
                serde_json::json!([
                    ModernUserOperationRequest::try_from(&*user_op)?,
                    entry_point_address
                ]),
            )
            .await?;
            let sponsored = response
                .result
                .ok_or_else(|| anyhow!("paymaster did not return sponsorship data"))?;
            apply_modern_sponsorship(user_op, &sponsored)
        }
    }
}

async fn send_user_operation(
    env: &Environment,
    http_client: &Client,
    user_op: &UserOperationData,
    entry_point_address: &str,
    variant: UserOperationVariant,
) -> Result<String> {
    let params = match variant {
        UserOperationVariant::LegacyV06 => {
            serde_json::json!([
                LegacyUserOperationRequest::from(user_op),
                entry_point_address
            ])
        }
        UserOperationVariant::PackedV07 => {
            serde_json::json!([
                ModernUserOperationRequest::try_from(user_op)?,
                entry_point_address
            ])
        }
    };
    let response: JsonRpcResponse<String> = post_json_rpc(
        http_client,
        &env.aa_bundler_rpc_url,
        "eth_sendUserOperation",
        params,
    )
    .await?;

    response
        .result
        .ok_or_else(|| anyhow!("bundler did not return a user-operation hash"))
}

async fn wait_for_user_operation_receipt(
    env: &Environment,
    http_client: &Client,
    user_op_hash: &str,
) -> Result<String> {
    let timeout = Duration::from_millis(env.aa_user_operation_timeout_ms);
    let poll_interval = Duration::from_millis(env.aa_user_operation_poll_interval_ms);
    let started = Instant::now();

    loop {
        let response: JsonRpcResponse<Option<UserOperationReceiptEnvelope>> = post_json_rpc(
            http_client,
            &env.aa_bundler_rpc_url,
            "eth_getUserOperationReceipt",
            serde_json::json!([user_op_hash]),
        )
        .await?;

        if let Some(result) = response.result.flatten() {
            let status = result
                .receipt
                .status
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case("0x1"))
                .unwrap_or(true);
            if !status || matches!(result.success, Some(false)) {
                return Err(anyhow!(
                    "smart-account user operation reverted: {}",
                    result.receipt.transaction_hash
                ));
            }

            return Ok(result.receipt.transaction_hash);
        }

        if started.elapsed() >= timeout {
            return Err(anyhow!(
                "timed out waiting for user-operation receipt after {}ms",
                env.aa_user_operation_timeout_ms
            ));
        }

        sleep(poll_interval).await;
    }
}

async fn user_operation_hash(
    provider: &Arc<Provider<Http>>,
    user_op: &UserOperationData,
    entry_point: Address,
    chain_id: U256,
    variant: UserOperationVariant,
) -> Result<[u8; 32]> {
    match variant {
        UserOperationVariant::LegacyV06 => {
            legacy_user_operation_hash(user_op, entry_point, chain_id)
        }
        UserOperationVariant::PackedV07 => {
            modern_user_operation_hash(provider, user_op, entry_point, chain_id)
                .await
                .map(|hash| hash.to_fixed_bytes())
        }
    }
}

fn legacy_user_operation_hash(
    user_op: &UserOperationData,
    entry_point: Address,
    chain_id: U256,
) -> Result<[u8; 32]> {
    let packed = encode(&[
        Token::Address(user_op.sender),
        Token::Uint(user_op.nonce),
        Token::FixedBytes(keccak256(&user_op.init_code).to_vec()),
        Token::FixedBytes(keccak256(&user_op.call_data).to_vec()),
        Token::Uint(user_op.call_gas_limit),
        Token::Uint(user_op.verification_gas_limit),
        Token::Uint(user_op.pre_verification_gas),
        Token::Uint(user_op.max_fee_per_gas),
        Token::Uint(user_op.max_priority_fee_per_gas),
        Token::FixedBytes(keccak256(&user_op.paymaster_and_data).to_vec()),
    ]);
    let user_op_pack_hash = keccak256(packed);
    Ok(keccak256(encode(&[
        Token::FixedBytes(user_op_pack_hash.to_vec()),
        Token::Address(entry_point),
        Token::Uint(chain_id),
    ])))
}

async fn modern_user_operation_hash(
    provider: &Arc<Provider<Http>>,
    user_op: &UserOperationData,
    entry_point: Address,
    chain_id: U256,
) -> Result<H256> {
    let account_gas_limits = pack_two_u128(user_op.verification_gas_limit, user_op.call_gas_limit)?;
    let gas_fees = pack_two_u128(user_op.max_priority_fee_per_gas, user_op.max_fee_per_gas)?;
    let packed_user_op_hash = H256::from(keccak256(encode(&[
        Token::FixedBytes(packed_user_operation_type_hash().to_vec()),
        Token::Address(user_op.sender),
        Token::Uint(user_op.nonce),
        Token::FixedBytes(keccak256(&user_op.init_code).to_vec()),
        Token::FixedBytes(keccak256(&user_op.call_data).to_vec()),
        Token::FixedBytes(account_gas_limits.as_bytes().to_vec()),
        Token::Uint(user_op.pre_verification_gas),
        Token::FixedBytes(gas_fees.as_bytes().to_vec()),
        Token::FixedBytes(keccak256(&user_op.paymaster_and_data).to_vec()),
    ])));

    if let Some(domain_separator) = entry_point_domain_separator(provider, entry_point).await? {
        return Ok(H256::from(keccak256(
            [
                &[0x19, 0x01][..],
                domain_separator.as_bytes(),
                packed_user_op_hash.as_bytes(),
            ]
            .concat(),
        )));
    }

    Ok(H256::from(keccak256(encode(&[
        Token::FixedBytes(packed_user_op_hash.to_fixed_bytes().to_vec()),
        Token::Address(entry_point),
        Token::Uint(chain_id),
    ]))))
}

async fn load_user_operation_gas_price(
    env: &Environment,
    http_client: &Client,
) -> Result<UserOperationGasPriceQuote> {
    let response: JsonRpcResponse<UserOperationGasPriceTiers> = post_json_rpc(
        http_client,
        &env.aa_bundler_rpc_url,
        "pimlico_getUserOperationGasPrice",
        serde_json::json!([]),
    )
    .await
    .context("failed to fetch user-operation gas price quote from AA_BUNDLER_RPC_URL")?;
    let tiers = response
        .result
        .ok_or_else(|| anyhow!("bundler did not return user-operation gas price tiers"))?;
    let selected = tiers
        .fast
        .or(tiers.standard)
        .or(tiers.slow)
        .ok_or_else(|| anyhow!("bundler returned no usable user-operation gas price tier"))?;

    Ok(UserOperationGasPriceQuote {
        max_fee_per_gas: parse_u256_hex(&selected.max_fee_per_gas)
            .context("bundler returned invalid maxFeePerGas")?,
        max_priority_fee_per_gas: parse_u256_hex(&selected.max_priority_fee_per_gas)
            .context("bundler returned invalid maxPriorityFeePerGas")?,
    })
}

async fn chain_provider(env: &Environment) -> Result<Arc<Provider<Http>>> {
    rpc::monad_provider_arc(env).await
}

fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|error| anyhow!("invalid address `{value}`: {error}"))
}

fn parse_u256_hex(value: &str) -> Result<U256> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("expected hex quantity but found empty string"));
    }

    let stripped = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if stripped.is_empty() {
        return Ok(U256::zero());
    }

    U256::from_str_radix(stripped, 16)
        .map_err(|error| anyhow!("invalid hex quantity `{value}`: {error}"))
}

fn parse_hex_bytes(value: &str) -> Result<Vec<u8>> {
    let stripped = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);

    if stripped.is_empty() {
        return Ok(Vec::new());
    }

    hex::decode(stripped).map_err(|error| anyhow!("invalid hex bytes `{value}`: {error}"))
}

fn bytes_to_hex<T: AsRef<[u8]>>(value: T) -> String {
    format!("0x{}", hex::encode(value))
}

fn u256_to_hex(value: U256) -> String {
    format!("{value:#x}")
}

fn dummy_user_operation_signature() -> &'static str {
    "0x000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000011b"
}

fn preferred_user_operation_variants(entry_point_address: &str) -> [UserOperationVariant; 2] {
    let entry_point_address = entry_point_address.trim().to_ascii_lowercase();
    if entry_point_address == "0x5ff137d4b0fdcd49dca30c7cf57e578a026d2789" {
        return [
            UserOperationVariant::LegacyV06,
            UserOperationVariant::PackedV07,
        ];
    }

    [
        UserOperationVariant::PackedV07,
        UserOperationVariant::LegacyV06,
    ]
}

fn is_user_operation_schema_mismatch(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("Unrecognized keys:")
        || message.contains("unknown field")
        || message.contains("invalid type")
        || message.contains("failed to deserialize")
}

fn split_init_code(init_code: &Bytes) -> Result<(Option<String>, Option<String>)> {
    if init_code.as_ref().is_empty() {
        return Ok((None, None));
    }

    let raw = init_code.as_ref();
    if raw.len() < 20 {
        return Err(anyhow!(
            "initCode must include a 20-byte factory address when using packed v0.7+ user operations"
        ));
    }

    let factory = Address::from_slice(&raw[..20]);
    let factory_data = Bytes::from(raw[20..].to_vec());
    Ok((
        Some(format!("{factory:#x}")),
        Some(bytes_to_hex(factory_data)),
    ))
}

fn split_paymaster_and_data(paymaster_and_data: &Bytes) -> Result<PaymasterFields> {
    if paymaster_and_data.as_ref().is_empty() {
        return Ok(PaymasterFields::default());
    }

    let raw = paymaster_and_data.as_ref();
    if raw.len() < 52 {
        return Err(anyhow!(
            "paymasterAndData must include paymaster address and gas limits when using packed v0.7+ user operations"
        ));
    }

    let paymaster = Address::from_slice(&raw[..20]);
    let verification_gas_limit = u128::from_be_bytes(
        raw[20..36]
            .try_into()
            .map_err(|_| anyhow!("invalid packed paymaster verification gas limit"))?,
    );
    let post_op_gas_limit = u128::from_be_bytes(
        raw[36..52]
            .try_into()
            .map_err(|_| anyhow!("invalid packed paymaster post-op gas limit"))?,
    );

    Ok(PaymasterFields {
        paymaster: Some(format!("{paymaster:#x}")),
        paymaster_verification_gas_limit: Some(format!("{verification_gas_limit:#x}")),
        paymaster_post_op_gas_limit: Some(format!("{post_op_gas_limit:#x}")),
        paymaster_data: Some(bytes_to_hex(Bytes::from(raw[52..].to_vec()))),
    })
}

fn pack_paymaster_and_data(
    paymaster: Address,
    verification_gas_limit: U256,
    post_op_gas_limit: U256,
    paymaster_data: Bytes,
) -> Result<Bytes> {
    let verification_gas_limit = u256_to_packed_u128_bytes(verification_gas_limit)?;
    let post_op_gas_limit = u256_to_packed_u128_bytes(post_op_gas_limit)?;

    let mut packed = Vec::with_capacity(20 + 16 + 16 + paymaster_data.len());
    packed.extend_from_slice(paymaster.as_bytes());
    packed.extend_from_slice(&verification_gas_limit);
    packed.extend_from_slice(&post_op_gas_limit);
    packed.extend_from_slice(paymaster_data.as_ref());
    Ok(Bytes::from(packed))
}

fn pack_two_u128(high: U256, low: U256) -> Result<H256> {
    let max = U256::from(u128::MAX);
    if high > max || low > max {
        return Err(anyhow!(
            "value exceeds uint128 range required by packed v0.7+ user operations"
        ));
    }

    let mut packed = [0_u8; 32];
    packed[..16].copy_from_slice(&high.as_u128().to_be_bytes());
    packed[16..].copy_from_slice(&low.as_u128().to_be_bytes());
    Ok(H256::from(packed))
}

fn u256_to_packed_u128_bytes(value: U256) -> Result<[u8; 16]> {
    if value > U256::from(u128::MAX) {
        return Err(anyhow!(
            "value exceeds uint128 range required by packed v0.7+ user operations"
        ));
    }

    Ok(value.as_u128().to_be_bytes())
}

async fn entry_point_domain_separator(
    provider: &Arc<Provider<Http>>,
    entry_point: Address,
) -> Result<Option<H256>> {
    let contract = Contract::new(entry_point, entry_point_v08_domain_abi()?, provider.clone());

    match contract
        .method::<_, H256>("getDomainSeparatorV4", ())?
        .call()
        .await
    {
        Ok(value) => Ok(Some(value)),
        Err(_) => Ok(None),
    }
}

fn packed_user_operation_type_hash() -> [u8; 32] {
    keccak256(
        "PackedUserOperation(address sender,uint256 nonce,bytes initCode,bytes callData,bytes32 accountGasLimits,uint256 preVerificationGas,bytes32 gasFees,bytes paymasterAndData)",
    )
}

fn entry_point_v08_domain_abi() -> Result<ethers_core::abi::Abi> {
    ethers_core::abi::AbiParser::default()
        .parse(&["function getDomainSeparatorV4() view returns (bytes32)"])
        .map_err(Into::into)
}

async fn post_json_rpc<T: for<'de> Deserialize<'de>>(
    http_client: &Client,
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<JsonRpcResponse<T>> {
    let response = http_client
        .post(url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }))
        .send()
        .await
        .with_context(|| format!("JSON-RPC request failed for `{method}`"))?;

    let body = response
        .error_for_status()
        .with_context(|| format!("JSON-RPC server returned HTTP error for `{method}`"))?
        .json::<JsonRpcResponse<T>>()
        .await
        .with_context(|| format!("failed to decode JSON-RPC response for `{method}`"))?;

    if let Some(error) = body.error {
        return Err(anyhow!(
            "JSON-RPC `{method}` failed with code {}: {}",
            error.code,
            error.message
        ));
    }

    Ok(body)
}

#[derive(Debug, Clone, Copy)]
enum UserOperationVariant {
    LegacyV06,
    PackedV07,
}

impl UserOperationVariant {
    fn label(self) -> &'static str {
        match self {
            Self::LegacyV06 => "legacy v0.6",
            Self::PackedV07 => "packed v0.7+",
        }
    }
}

#[derive(Debug, Clone)]
struct UserOperationData {
    sender: Address,
    nonce: U256,
    init_code: Bytes,
    call_data: Bytes,
    call_gas_limit: U256,
    verification_gas_limit: U256,
    pre_verification_gas: U256,
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
    paymaster_and_data: Bytes,
    signature: String,
}

#[derive(Debug)]
struct UserOperationGasPriceQuote {
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
}

#[derive(Debug, Deserialize)]
struct UserOperationGasPriceTiers {
    #[serde(default)]
    slow: Option<UserOperationGasPriceTier>,
    #[serde(default)]
    standard: Option<UserOperationGasPriceTier>,
    #[serde(default)]
    fast: Option<UserOperationGasPriceTier>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserOperationGasPriceTier {
    max_fee_per_gas: String,
    max_priority_fee_per_gas: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyUserOperationRequest {
    sender: String,
    nonce: String,
    init_code: String,
    call_data: String,
    call_gas_limit: String,
    verification_gas_limit: String,
    pre_verification_gas: String,
    max_fee_per_gas: String,
    max_priority_fee_per_gas: String,
    paymaster_and_data: String,
    signature: String,
}

impl From<&UserOperationData> for LegacyUserOperationRequest {
    fn from(value: &UserOperationData) -> Self {
        Self {
            sender: format!("{:#x}", value.sender),
            nonce: u256_to_hex(value.nonce),
            init_code: bytes_to_hex(&value.init_code),
            call_data: bytes_to_hex(&value.call_data),
            call_gas_limit: u256_to_hex(value.call_gas_limit),
            verification_gas_limit: u256_to_hex(value.verification_gas_limit),
            pre_verification_gas: u256_to_hex(value.pre_verification_gas),
            max_fee_per_gas: u256_to_hex(value.max_fee_per_gas),
            max_priority_fee_per_gas: u256_to_hex(value.max_priority_fee_per_gas),
            paymaster_and_data: bytes_to_hex(&value.paymaster_and_data),
            signature: value.signature.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModernUserOperationRequest {
    sender: String,
    nonce: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    factory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    factory_data: Option<String>,
    call_data: String,
    call_gas_limit: String,
    verification_gas_limit: String,
    pre_verification_gas: String,
    max_fee_per_gas: String,
    max_priority_fee_per_gas: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    paymaster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    paymaster_verification_gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    paymaster_post_op_gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    paymaster_data: Option<String>,
    signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacySponsoredUserOperation {
    call_gas_limit: String,
    verification_gas_limit: String,
    pre_verification_gas: String,
    paymaster_and_data: String,
    max_fee_per_gas: Option<String>,
    max_priority_fee_per_gas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModernSponsoredUserOperation {
    paymaster: String,
    paymaster_data: String,
    pre_verification_gas: String,
    verification_gas_limit: String,
    call_gas_limit: String,
    paymaster_verification_gas_limit: String,
    paymaster_post_op_gas_limit: String,
    max_fee_per_gas: Option<String>,
    max_priority_fee_per_gas: Option<String>,
}

#[derive(Debug, Default)]
struct PaymasterFields {
    paymaster: Option<String>,
    paymaster_verification_gas_limit: Option<String>,
    paymaster_post_op_gas_limit: Option<String>,
    paymaster_data: Option<String>,
}

impl TryFrom<&UserOperationData> for ModernUserOperationRequest {
    type Error = anyhow::Error;

    fn try_from(value: &UserOperationData) -> Result<Self, Self::Error> {
        let (factory, factory_data) = split_init_code(&value.init_code)?;
        let paymaster_fields = split_paymaster_and_data(&value.paymaster_and_data)?;

        Ok(Self {
            sender: format!("{:#x}", value.sender),
            nonce: u256_to_hex(value.nonce),
            factory,
            factory_data,
            call_data: bytes_to_hex(&value.call_data),
            call_gas_limit: u256_to_hex(value.call_gas_limit),
            verification_gas_limit: u256_to_hex(value.verification_gas_limit),
            pre_verification_gas: u256_to_hex(value.pre_verification_gas),
            max_fee_per_gas: u256_to_hex(value.max_fee_per_gas),
            max_priority_fee_per_gas: u256_to_hex(value.max_priority_fee_per_gas),
            paymaster: paymaster_fields.paymaster,
            paymaster_verification_gas_limit: paymaster_fields.paymaster_verification_gas_limit,
            paymaster_post_op_gas_limit: paymaster_fields.paymaster_post_op_gas_limit,
            paymaster_data: paymaster_fields.paymaster_data,
            signature: value.signature.clone(),
        })
    }
}

fn apply_legacy_sponsorship(
    user_op: &mut UserOperationData,
    sponsored: &LegacySponsoredUserOperation,
) -> Result<()> {
    user_op.call_gas_limit = parse_u256_hex(&sponsored.call_gas_limit)
        .context("paymaster returned invalid callGasLimit")?;
    user_op.verification_gas_limit = parse_u256_hex(&sponsored.verification_gas_limit)
        .context("paymaster returned invalid verificationGasLimit")?;
    user_op.pre_verification_gas = parse_u256_hex(&sponsored.pre_verification_gas)
        .context("paymaster returned invalid preVerificationGas")?;
    user_op.paymaster_and_data = Bytes::from(
        parse_hex_bytes(&sponsored.paymaster_and_data)
            .context("paymaster returned invalid paymasterAndData")?,
    );
    if let Some(value) = sponsored.max_fee_per_gas.as_deref() {
        user_op.max_fee_per_gas =
            parse_u256_hex(value).context("paymaster returned invalid maxFeePerGas")?;
    }
    if let Some(value) = sponsored.max_priority_fee_per_gas.as_deref() {
        user_op.max_priority_fee_per_gas =
            parse_u256_hex(value).context("paymaster returned invalid maxPriorityFeePerGas")?;
    }

    Ok(())
}

fn apply_modern_sponsorship(
    user_op: &mut UserOperationData,
    sponsored: &ModernSponsoredUserOperation,
) -> Result<()> {
    user_op.call_gas_limit = parse_u256_hex(&sponsored.call_gas_limit)
        .context("paymaster returned invalid callGasLimit")?;
    user_op.verification_gas_limit = parse_u256_hex(&sponsored.verification_gas_limit)
        .context("paymaster returned invalid verificationGasLimit")?;
    user_op.pre_verification_gas = parse_u256_hex(&sponsored.pre_verification_gas)
        .context("paymaster returned invalid preVerificationGas")?;
    user_op.paymaster_and_data = pack_paymaster_and_data(
        parse_address(&sponsored.paymaster).context("paymaster returned invalid paymaster")?,
        parse_u256_hex(&sponsored.paymaster_verification_gas_limit)
            .context("paymaster returned invalid paymasterVerificationGasLimit")?,
        parse_u256_hex(&sponsored.paymaster_post_op_gas_limit)
            .context("paymaster returned invalid paymasterPostOpGasLimit")?,
        Bytes::from(
            parse_hex_bytes(&sponsored.paymaster_data)
                .context("paymaster returned invalid paymasterData")?,
        ),
    )?;
    if let Some(value) = sponsored.max_fee_per_gas.as_deref() {
        user_op.max_fee_per_gas =
            parse_u256_hex(value).context("paymaster returned invalid maxFeePerGas")?;
    }
    if let Some(value) = sponsored.max_priority_fee_per_gas.as_deref() {
        user_op.max_priority_fee_per_gas =
            parse_u256_hex(value).context("paymaster returned invalid maxPriorityFeePerGas")?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserOperationReceiptEnvelope {
    success: Option<bool>,
    receipt: TransactionReceiptSummary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionReceiptSummary {
    transaction_hash: String,
    status: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::user_operation_salt;
    use uuid::Uuid;

    #[test]
    fn derives_non_zero_salt_from_user_id() {
        let salt =
            user_operation_salt(Uuid::parse_str("7d444840-9dc0-11d1-b245-5ffdce74fad2").unwrap());

        assert!(!salt.is_zero());
    }
}
