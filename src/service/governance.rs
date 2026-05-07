use anyhow::Result;
use ethers_contract::Contract;
use ethers_core::{
    abi::{Abi, Tokenize},
    types::{Address, Bytes, H256, U256},
    utils::keccak256,
};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::LocalWallet;

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        admin::schema::{
            AdminAccessControlOverviewResponse, AdminAccessControlRoleMembershipResponse,
            AdminAccessControlRoleSummaryResponse, AdminAccessControlRoleWriteRequest,
            AdminAccessControlRoleWriteResponse, AdminMultiSigOverviewResponse,
            AdminMultiSigProposalRequest, AdminMultiSigProposalResponse,
            AdminMultiSigProposalSignatureResponse, AdminMultiSigProposalWriteResponse,
            AdminMultiSigQuorumWriteRequest, AdminMultiSigSignerWriteRequest,
        },
        auth::error::AuthError,
    },
    service::{
        chain::{
            admin_signer, format_address, format_h256, parse_address, parse_bytes_input,
            parse_contract_address, parse_u256, wait_for_receipt,
        },
        rpc,
    },
};

const ACCESS_CONTROL_ABI_JSON: &str = r#"
[
  { "type": "function", "name": "DEFAULT_ADMIN_ROLE", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "ADMIN_ROLE", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "ISSUER_ROLE", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "COMPLIANCE_ROLE", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "ORACLE_ROLE", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "OPERATOR_ROLE", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "PAUSER_ROLE", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "TREASURY_ROLE", "inputs": [], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "hasRole", "inputs": [
    { "name": "role", "type": "bytes32", "internalType": "bytes32" },
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "bool", "internalType": "bool" }], "stateMutability": "view" },
  { "type": "function", "name": "getRoleAdmin", "inputs": [
    { "name": "role", "type": "bytes32", "internalType": "bytes32" }
  ], "outputs": [{ "name": "", "type": "bytes32", "internalType": "bytes32" }], "stateMutability": "view" },
  { "type": "function", "name": "grantRole", "inputs": [
    { "name": "role", "type": "bytes32", "internalType": "bytes32" },
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "revokeRole", "inputs": [
    { "name": "role", "type": "bytes32", "internalType": "bytes32" },
    { "name": "account", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" }
]
"#;

const MULTISIG_ABI_JSON: &str = r#"
[
  { "type": "function", "name": "getSigners", "inputs": [], "outputs": [{ "name": "", "type": "address[]", "internalType": "address[]" }], "stateMutability": "view" },
  { "type": "function", "name": "quorum", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "proposalCount", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "timelockDuration", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "PROPOSAL_EXPIRY", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "MIN_TIMELOCK", "inputs": [], "outputs": [{ "name": "", "type": "uint256", "internalType": "uint256" }], "stateMutability": "view" },
  { "type": "function", "name": "isSigner", "inputs": [{ "name": "account", "type": "address", "internalType": "address" }], "outputs": [{ "name": "", "type": "bool", "internalType": "bool" }], "stateMutability": "view" },
  { "type": "function", "name": "hasSignedProposal", "inputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" },
    { "name": "signer", "type": "address", "internalType": "address" }
  ], "outputs": [{ "name": "", "type": "bool", "internalType": "bool" }], "stateMutability": "view" },
  { "type": "function", "name": "getProposalState", "inputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [
    { "name": "target", "type": "address", "internalType": "address" },
    { "name": "executed", "type": "bool", "internalType": "bool" },
    { "name": "cancelled", "type": "bool", "internalType": "bool" },
    { "name": "signaturesCount", "type": "uint256", "internalType": "uint256" },
    { "name": "expiresAt", "type": "uint256", "internalType": "uint256" },
    { "name": "timelockUntil", "type": "uint256", "internalType": "uint256" },
    { "name": "readyToExecute", "type": "bool", "internalType": "bool" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "proposals", "inputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [
    { "name": "proposalHash", "type": "bytes32", "internalType": "bytes32" },
    { "name": "target", "type": "address", "internalType": "address" },
    { "name": "data", "type": "bytes", "internalType": "bytes" },
    { "name": "value", "type": "uint256", "internalType": "uint256" },
    { "name": "signaturesCount", "type": "uint256", "internalType": "uint256" },
    { "name": "createdAt", "type": "uint256", "internalType": "uint256" },
    { "name": "expiresAt", "type": "uint256", "internalType": "uint256" },
    { "name": "timelockUntil", "type": "uint256", "internalType": "uint256" },
    { "name": "executed", "type": "bool", "internalType": "bool" },
    { "name": "cancelled", "type": "bool", "internalType": "bool" },
    { "name": "proposer", "type": "address", "internalType": "address" }
  ], "stateMutability": "view" },
  { "type": "function", "name": "propose", "inputs": [
    { "name": "target", "type": "address", "internalType": "address" },
    { "name": "data", "type": "bytes", "internalType": "bytes" },
    { "name": "value", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [{ "name": "proposalId", "type": "uint256", "internalType": "uint256" }], "stateMutability": "nonpayable" },
  { "type": "function", "name": "sign", "inputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "execute", "inputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "cancel", "inputs": [
    { "name": "proposalId", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "addSigner", "inputs": [
    { "name": "newSigner", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "removeSigner", "inputs": [
    { "name": "signerToRemove", "type": "address", "internalType": "address" }
  ], "outputs": [], "stateMutability": "nonpayable" },
  { "type": "function", "name": "updateQuorum", "inputs": [
    { "name": "newQuorum", "type": "uint256", "internalType": "uint256" }
  ], "outputs": [], "stateMutability": "nonpayable" }
]
"#;

type ProposalTuple = (
    H256,
    Address,
    Bytes,
    U256,
    U256,
    U256,
    U256,
    U256,
    bool,
    bool,
    Address,
);

type ProposalStateTuple = (Address, bool, bool, U256, U256, U256, bool);

pub async fn get_access_control_overview(
    state: &AppState,
) -> Result<AdminAccessControlOverviewResponse, AuthError> {
    let contract = read_access_control_contract(&state.env)
        .await
        .map_err(|error| {
            AuthError::internal("failed to build access control read contract", error)
        })?;

    let mut roles = Vec::with_capacity(known_roles().len());
    for (label, role) in known_roles() {
        let admin_role = contract
            .method::<_, H256>("getRoleAdmin", role)
            .map_err(|error| AuthError::internal("failed to build getRoleAdmin call", error))?
            .call()
            .await
            .map_err(|error| AuthError::internal("failed to call getRoleAdmin", error))?;
        roles.push(AdminAccessControlRoleSummaryResponse {
            role: label.to_owned(),
            role_hex: format_h256(role),
            admin_role: role_label(admin_role).unwrap_or("CUSTOM_ROLE").to_owned(),
            admin_role_hex: format_h256(admin_role),
        });
    }

    Ok(AdminAccessControlOverviewResponse {
        access_control_address: state.env.access_control_address.clone(),
        roles,
    })
}

pub async fn get_access_control_role_membership(
    state: &AppState,
    role: &str,
    account: &str,
) -> Result<AdminAccessControlRoleMembershipResponse, AuthError> {
    let role = parse_role_input(role)?;
    let account = parse_address(account)?;
    let contract = read_access_control_contract(&state.env)
        .await
        .map_err(|error| {
            AuthError::internal("failed to build access control read contract", error)
        })?;

    let has_role = contract
        .method::<_, bool>("hasRole", (role, account))
        .map_err(|error| AuthError::internal("failed to build hasRole call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call hasRole", error))?;
    let admin_role = contract
        .method::<_, H256>("getRoleAdmin", role)
        .map_err(|error| AuthError::internal("failed to build getRoleAdmin call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getRoleAdmin", error))?;

    Ok(AdminAccessControlRoleMembershipResponse {
        access_control_address: state.env.access_control_address.clone(),
        account_address: format_address(account),
        role: role_label(role).unwrap_or("CUSTOM_ROLE").to_owned(),
        role_hex: format_h256(role),
        has_role,
        admin_role: role_label(admin_role).unwrap_or("CUSTOM_ROLE").to_owned(),
        admin_role_hex: format_h256(admin_role),
    })
}

pub async fn grant_access_control_role(
    state: &AppState,
    payload: AdminAccessControlRoleWriteRequest,
) -> Result<AdminAccessControlRoleWriteResponse, AuthError> {
    write_access_control_role(state, payload, "grantRole", "granted").await
}

pub async fn revoke_access_control_role(
    state: &AppState,
    payload: AdminAccessControlRoleWriteRequest,
) -> Result<AdminAccessControlRoleWriteResponse, AuthError> {
    write_access_control_role(state, payload, "revokeRole", "revoked").await
}

pub async fn get_multisig_overview(
    state: &AppState,
) -> Result<AdminMultiSigOverviewResponse, AuthError> {
    let contract = read_multisig_contract(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build multisig read contract", error))?;

    let signers = contract
        .method::<_, Vec<Address>>("getSigners", ())
        .map_err(|error| AuthError::internal("failed to build getSigners call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getSigners", error))?;
    let quorum = contract
        .method::<_, U256>("quorum", ())
        .map_err(|error| AuthError::internal("failed to build quorum call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call quorum", error))?;
    let proposal_count = contract
        .method::<_, U256>("proposalCount", ())
        .map_err(|error| AuthError::internal("failed to build proposalCount call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call proposalCount", error))?;
    let timelock_duration = contract
        .method::<_, U256>("timelockDuration", ())
        .map_err(|error| AuthError::internal("failed to build timelockDuration call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call timelockDuration", error))?;
    let proposal_expiry = contract
        .method::<_, U256>("PROPOSAL_EXPIRY", ())
        .map_err(|error| AuthError::internal("failed to build PROPOSAL_EXPIRY call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call PROPOSAL_EXPIRY", error))?;
    let min_timelock = contract
        .method::<_, U256>("MIN_TIMELOCK", ())
        .map_err(|error| AuthError::internal("failed to build MIN_TIMELOCK call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call MIN_TIMELOCK", error))?;

    Ok(AdminMultiSigOverviewResponse {
        multisig_address: multisig_address_string(&state.env)?,
        signers: signers.into_iter().map(format_address).collect(),
        quorum: quorum.to_string(),
        proposal_count: proposal_count.to_string(),
        timelock_duration: timelock_duration.to_string(),
        proposal_expiry: proposal_expiry.to_string(),
        min_timelock: min_timelock.to_string(),
    })
}

pub async fn get_multisig_proposal(
    state: &AppState,
    proposal_id: &str,
) -> Result<AdminMultiSigProposalResponse, AuthError> {
    read_multisig_proposal(state, parse_u256(proposal_id, "proposal_id")?).await
}

pub async fn get_multisig_proposal_signature(
    state: &AppState,
    proposal_id: &str,
    signer: &str,
) -> Result<AdminMultiSigProposalSignatureResponse, AuthError> {
    let proposal_id = parse_u256(proposal_id, "proposal_id")?;
    let signer = parse_address(signer)?;
    let contract = read_multisig_contract(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build multisig read contract", error))?;

    let has_signed = contract
        .method::<_, bool>("hasSignedProposal", (proposal_id, signer))
        .map_err(|error| AuthError::internal("failed to build hasSignedProposal call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call hasSignedProposal", error))?;

    Ok(AdminMultiSigProposalSignatureResponse {
        multisig_address: multisig_address_string(&state.env)?,
        proposal_id: proposal_id.to_string(),
        signer_address: format_address(signer),
        has_signed,
    })
}

pub async fn propose_multisig_transaction(
    state: &AppState,
    payload: AdminMultiSigProposalRequest,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    let target = parse_address(&payload.target)?;
    let data = parse_bytes_input(Some(&payload.data), "data")?;
    let value = parse_u256(payload.value.as_deref().unwrap_or("0"), "value")?;

    submit_multisig_proposal(state, target, data, value).await
}

pub async fn propose_multisig_add_signer(
    state: &AppState,
    payload: AdminMultiSigSignerWriteRequest,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    let signer = parse_address(&payload.signer_address)?;
    propose_multisig_self_call(state, "addSigner", signer).await
}

pub async fn propose_multisig_remove_signer(
    state: &AppState,
    payload: AdminMultiSigSignerWriteRequest,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    let signer = parse_address(&payload.signer_address)?;
    propose_multisig_self_call(state, "removeSigner", signer).await
}

pub async fn propose_multisig_update_quorum(
    state: &AppState,
    payload: AdminMultiSigQuorumWriteRequest,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    let quorum = parse_u256(&payload.quorum, "quorum")?;
    propose_multisig_self_call(state, "updateQuorum", quorum).await
}

pub async fn sign_multisig_proposal(
    state: &AppState,
    proposal_id: &str,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    write_multisig_proposal_action(
        state,
        proposal_id,
        "sign",
        "failed to submit multisig sign transaction",
    )
    .await
}

pub async fn execute_multisig_proposal(
    state: &AppState,
    proposal_id: &str,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    write_multisig_proposal_action(
        state,
        proposal_id,
        "execute",
        "failed to submit multisig execute transaction",
    )
    .await
}

pub async fn cancel_multisig_proposal(
    state: &AppState,
    proposal_id: &str,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    write_multisig_proposal_action(
        state,
        proposal_id,
        "cancel",
        "failed to submit multisig cancel transaction",
    )
    .await
}

async fn write_access_control_role(
    state: &AppState,
    payload: AdminAccessControlRoleWriteRequest,
    method: &str,
    action: &str,
) -> Result<AdminAccessControlRoleWriteResponse, AuthError> {
    let role = parse_role_input(&payload.role)?;
    let account = parse_address(&payload.account_address)?;
    log_access_control_write_preflight(&state.env, method, role, account).await?;

    let tx_hash = send_access_control_transaction::<_, ()>(
        &state.env,
        method,
        (role, account),
        "failed to submit access control role transaction",
    )
    .await?;

    let membership =
        get_access_control_role_membership(state, &format_h256(role), &format_address(account))
            .await?;

    Ok(AdminAccessControlRoleWriteResponse {
        tx_hash,
        action: action.to_owned(),
        membership,
    })
}

async fn log_access_control_write_preflight(
    env: &Environment,
    method: &str,
    role: H256,
    account: Address,
) -> Result<Address, AuthError> {
    let signer = admin_signer(env).await?;
    let signer_address = signer.address();
    let contract = read_access_control_contract(env).await.map_err(|error| {
        AuthError::internal("failed to build access control read contract", error)
    })?;

    let admin_role = contract
        .method::<_, H256>("getRoleAdmin", role)
        .map_err(|error| AuthError::internal("failed to build getRoleAdmin call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getRoleAdmin", error))?;
    let signer_has_admin_role = contract
        .method::<_, bool>("hasRole", (admin_role, signer_address))
        .map_err(|error| AuthError::internal("failed to build signer hasRole call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call signer hasRole", error))?;
    let account_has_role = contract
        .method::<_, bool>("hasRole", (role, account))
        .map_err(|error| AuthError::internal("failed to build account hasRole call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call account hasRole", error))?;

    tracing::info!(
        method,
        access_control_address = %env.access_control_address,
        signer_address = %format_address(signer_address),
        role = role_label(role).unwrap_or("CUSTOM_ROLE"),
        role_hex = %format_h256(role),
        admin_role = role_label(admin_role).unwrap_or("CUSTOM_ROLE"),
        admin_role_hex = %format_h256(admin_role),
        target_account = %format_address(account),
        signer_has_admin_role,
        account_has_role,
        "access control write preflight"
    );

    let call = contract
        .method::<_, ()>(method, (role, account))
        .map_err(|error| {
            AuthError::internal("failed to build access control preflight call", error)
        })?
        .from(signer_address);
    if let Err(error) = call.call().await {
        tracing::error!(
            %error,
            ?error,
            method,
            access_control_address = %env.access_control_address,
            signer_address = %format_address(signer_address),
            role = role_label(role).unwrap_or("CUSTOM_ROLE"),
            role_hex = %format_h256(role),
            admin_role = role_label(admin_role).unwrap_or("CUSTOM_ROLE"),
            admin_role_hex = %format_h256(admin_role),
            target_account = %format_address(account),
            signer_has_admin_role,
            account_has_role,
            "access control preflight reverted"
        );

        let reason = if signer_has_admin_role {
            format!(
                "{} preflight reverted for backend signer {} on access control {}",
                method,
                format_address(signer_address),
                env.access_control_address
            )
        } else {
            format!(
                "backend signer {} lacks {} ({}) on access control {} required for {}",
                format_address(signer_address),
                role_label(admin_role).unwrap_or("CUSTOM_ROLE"),
                format_h256(admin_role),
                env.access_control_address,
                method
            )
        };
        return Err(AuthError::forbidden(reason));
    }

    Ok(signer_address)
}

async fn write_multisig_proposal_action(
    state: &AppState,
    proposal_id: &str,
    method: &str,
    error_context: &'static str,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    let proposal_id = parse_u256(proposal_id, "proposal_id")?;
    let tx_hash =
        send_multisig_transaction::<_, ()>(&state.env, method, proposal_id, error_context).await?;
    let proposal = read_multisig_proposal(state, proposal_id).await?;

    Ok(AdminMultiSigProposalWriteResponse { tx_hash, proposal })
}

async fn submit_multisig_proposal(
    state: &AppState,
    target: Address,
    data: Bytes,
    value: U256,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError> {
    let proposal_id = next_multisig_proposal_id(&state.env).await?;
    let tx_hash = send_multisig_transaction::<_, U256>(
        &state.env,
        "propose",
        (target, data, value),
        "failed to submit multisig propose transaction",
    )
    .await?;
    let proposal = read_multisig_proposal(state, proposal_id).await?;

    Ok(AdminMultiSigProposalWriteResponse { tx_hash, proposal })
}

async fn propose_multisig_self_call<T>(
    state: &AppState,
    method: &str,
    args: T,
) -> Result<AdminMultiSigProposalWriteResponse, AuthError>
where
    T: Tokenize,
{
    let target = multisig_address(&state.env)?;
    let data = encode_multisig_call(method, args)?;
    submit_multisig_proposal(state, target, data, U256::zero()).await
}

async fn read_multisig_proposal(
    state: &AppState,
    proposal_id: U256,
) -> Result<AdminMultiSigProposalResponse, AuthError> {
    let contract = read_multisig_contract(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build multisig read contract", error))?;

    let (
        proposal_hash,
        target,
        data,
        value,
        signatures_count,
        created_at,
        expires_at,
        timelock_until,
        executed,
        cancelled,
        proposer,
    ) = contract
        .method::<_, ProposalTuple>("proposals", proposal_id)
        .map_err(|error| AuthError::internal("failed to build proposals call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call proposals", error))?;

    let (_, _, _, _, _, _, ready_to_execute) = contract
        .method::<_, ProposalStateTuple>("getProposalState", proposal_id)
        .map_err(|error| AuthError::internal("failed to build getProposalState call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call getProposalState", error))?;

    Ok(AdminMultiSigProposalResponse {
        multisig_address: multisig_address_string(&state.env)?,
        proposal_id: proposal_id.to_string(),
        proposal_hash: format_h256(proposal_hash),
        target: format_address(target),
        data: format!("0x{}", hex::encode(data.as_ref())),
        value: value.to_string(),
        signatures_count: signatures_count.to_string(),
        created_at: u256_to_i64(created_at, "created_at")?,
        expires_at: u256_to_i64(expires_at, "expires_at")?,
        timelock_until: u256_to_i64(timelock_until, "timelock_until")?,
        executed,
        cancelled,
        proposer: format_address(proposer),
        ready_to_execute,
    })
}

async fn read_access_control_contract(env: &Environment) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.access_control_address)?,
        access_control_abi()?,
        provider,
    ))
}

async fn write_access_control_contract(
    env: &Environment,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        parse_contract_address(&env.access_control_address)
            .map_err(|error| AuthError::internal("invalid access control address", error))?,
        access_control_abi()
            .map_err(|error| AuthError::internal("failed to build access control ABI", error))?,
        signer,
    ))
}

async fn send_access_control_transaction<T, D>(
    env: &Environment,
    method: &str,
    args: T,
    error_context: &'static str,
) -> Result<String, AuthError>
where
    T: ethers_core::abi::Tokenize,
    D: ethers_core::abi::Detokenize,
{
    let contract = write_access_control_contract(env).await?;
    let call = contract.method::<_, D>(method, args).map_err(|error| {
        AuthError::internal("failed to build access control transaction", error)
    })?;
    let pending = call
        .send()
        .await
        .map_err(|error| AuthError::internal(error_context, error))?;
    wait_for_receipt(pending).await
}

async fn read_multisig_contract(env: &Environment) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(
        multisig_address(env)?,
        multisig_abi()?,
        provider,
    ))
}

async fn write_multisig_contract(
    env: &Environment,
) -> Result<Contract<SignerMiddleware<Provider<Http>, LocalWallet>>, AuthError> {
    let signer = admin_signer(env).await?;
    Ok(Contract::new(
        multisig_address(env)
            .map_err(|error| AuthError::internal("invalid multisig address", error))?,
        multisig_abi()
            .map_err(|error| AuthError::internal("failed to build multisig ABI", error))?,
        signer,
    ))
}

async fn send_multisig_transaction<T, D>(
    env: &Environment,
    method: &str,
    args: T,
    error_context: &'static str,
) -> Result<String, AuthError>
where
    T: ethers_core::abi::Tokenize,
    D: ethers_core::abi::Detokenize,
{
    let contract = write_multisig_contract(env).await?;
    let call = contract
        .method::<_, D>(method, args)
        .map_err(|error| AuthError::internal("failed to build multisig transaction", error))?;
    let pending = call
        .send()
        .await
        .map_err(|error| AuthError::internal(error_context, error))?;
    wait_for_receipt(pending).await
}

fn access_control_abi() -> Result<Abi> {
    serde_json::from_str(ACCESS_CONTROL_ABI_JSON).map_err(Into::into)
}

fn multisig_abi() -> Result<Abi> {
    serde_json::from_str(MULTISIG_ABI_JSON).map_err(Into::into)
}

fn multisig_address(env: &Environment) -> Result<Address, AuthError> {
    let raw = env.admin_multisig_address.as_deref().ok_or_else(|| {
        AuthError::service_unavailable("ADMIN_MULTISIG_ADDRESS is not configured")
    })?;
    parse_contract_address(raw)
        .map_err(|error| AuthError::internal("invalid multisig address", error))
}

fn multisig_address_string(env: &Environment) -> Result<String, AuthError> {
    Ok(format_address(multisig_address(env)?))
}

async fn next_multisig_proposal_id(env: &Environment) -> Result<U256, AuthError> {
    let contract = read_multisig_contract(env)
        .await
        .map_err(|error| AuthError::internal("failed to build multisig read contract", error))?;
    contract
        .method::<_, U256>("proposalCount", ())
        .map_err(|error| AuthError::internal("failed to build proposalCount call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call proposalCount", error))
}

fn encode_multisig_call<T>(method: &str, args: T) -> Result<Bytes, AuthError>
where
    T: Tokenize,
{
    let abi = multisig_abi()
        .map_err(|error| AuthError::internal("failed to build multisig ABI", error))?;
    let function = abi
        .function(method)
        .map_err(|error| AuthError::internal("failed to load multisig ABI function", error))?;
    let encoded = function
        .encode_input(&args.into_tokens())
        .map_err(|error| AuthError::internal("failed to encode multisig calldata", error))?;
    Ok(Bytes::from(encoded))
}

fn parse_role_input(raw: &str) -> Result<H256, AuthError> {
    let normalized = raw.trim().to_ascii_uppercase();
    if let Some((_, value)) = known_roles().iter().find(|(label, _)| *label == normalized) {
        return Ok(*value);
    }

    parse_role_hex(raw)
}

fn parse_role_hex(raw: &str) -> Result<H256, AuthError> {
    let value = raw.trim();
    let stripped = value.strip_prefix("0x").ok_or_else(|| {
        AuthError::bad_request("role must be a known label or 0x-prefixed bytes32")
    })?;
    let bytes =
        hex::decode(stripped).map_err(|_| AuthError::bad_request("invalid role hex value"))?;
    if bytes.len() != 32 {
        return Err(AuthError::bad_request("role hex value must be 32 bytes"));
    }
    Ok(H256::from_slice(&bytes))
}

fn role_label(role: H256) -> Option<&'static str> {
    known_roles()
        .iter()
        .find_map(|(label, value)| (*value == role).then_some(*label))
}

fn known_roles() -> [(&'static str, H256); 8] {
    [
        ("DEFAULT_ADMIN_ROLE", H256::zero()),
        ("ADMIN_ROLE", hash_role("ADMIN_ROLE")),
        ("ISSUER_ROLE", hash_role("ISSUER_ROLE")),
        ("COMPLIANCE_ROLE", hash_role("COMPLIANCE_ROLE")),
        ("ORACLE_ROLE", hash_role("ORACLE_ROLE")),
        ("OPERATOR_ROLE", hash_role("OPERATOR_ROLE")),
        ("PAUSER_ROLE", hash_role("PAUSER_ROLE")),
        ("TREASURY_ROLE", hash_role("TREASURY_ROLE")),
    ]
}

fn hash_role(value: &str) -> H256 {
    H256::from(keccak256(value.as_bytes()))
}

fn u256_to_i64(value: U256, field_name: &str) -> Result<i64, AuthError> {
    i64::try_from(value)
        .map_err(|_| AuthError::bad_request(format!("{field_name} is out of range")))
}
