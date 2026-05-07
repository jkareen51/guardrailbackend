use axum::{
    Json,
    extract::{Extension, Multipart, Path, State},
};

use crate::{
    app::AppState,
    module::{
        admin::{
            crud,
            schema::{
                AdminAccessControlOverviewResponse, AdminAccessControlRoleMembershipResponse,
                AdminAccessControlRoleWriteRequest, AdminAccessControlRoleWriteResponse,
                AdminAuthResponse, AdminImageUploadResponse, AdminMeResponse,
                AdminMultiSigOverviewResponse, AdminMultiSigProposalRequest,
                AdminMultiSigProposalResponse, AdminMultiSigProposalSignatureResponse,
                AdminMultiSigProposalWriteResponse, AdminMultiSigQuorumWriteRequest,
                AdminMultiSigSignerWriteRequest, AdminWalletChallengeRequest,
                AdminWalletChallengeResponse, AdminWalletConnectRequest,
            },
        },
        auth::error::AuthError,
    },
    service::{
        admin_auth::{connect_wallet, create_wallet_challenge},
        governance,
        jwt::AuthenticatedUser,
        upload::upload_admin_image,
    },
};

pub async fn wallet_challenge(
    State(state): State<AppState>,
    Json(payload): Json<AdminWalletChallengeRequest>,
) -> Result<Json<AdminWalletChallengeResponse>, AuthError> {
    Ok(Json(create_wallet_challenge(&state, payload).await?))
}

pub async fn wallet_connect(
    State(state): State<AppState>,
    Json(payload): Json<AdminWalletConnectRequest>,
) -> Result<Json<AdminAuthResponse>, AuthError> {
    Ok(Json(connect_wallet(&state, payload).await?))
}

pub async fn me(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<AdminMeResponse>, AuthError> {
    let profile = crud::get_admin_profile(&state.db, authenticated_user.user_id).await?;

    Ok(Json(AdminMeResponse::from_profile(
        profile,
        state.env.monad_chain_id,
    )))
}

pub async fn upload_image(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    multipart: Multipart,
) -> Result<Json<AdminImageUploadResponse>, AuthError> {
    Ok(Json(
        upload_admin_image(&state, authenticated_user, multipart).await?,
    ))
}

pub async fn get_access_control_overview(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<AdminAccessControlOverviewResponse>, AuthError> {
    Ok(Json(governance::get_access_control_overview(&state).await?))
}

pub async fn get_access_control_role_membership(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Path((role, account)): Path<(String, String)>,
) -> Result<Json<AdminAccessControlRoleMembershipResponse>, AuthError> {
    Ok(Json(
        governance::get_access_control_role_membership(&state, &role, &account).await?,
    ))
}

pub async fn grant_access_control_role(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminAccessControlRoleWriteRequest>,
) -> Result<Json<AdminAccessControlRoleWriteResponse>, AuthError> {
    Ok(Json(
        governance::grant_access_control_role(&state, payload).await?,
    ))
}

pub async fn revoke_access_control_role(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminAccessControlRoleWriteRequest>,
) -> Result<Json<AdminAccessControlRoleWriteResponse>, AuthError> {
    Ok(Json(
        governance::revoke_access_control_role(&state, payload).await?,
    ))
}

pub async fn get_multisig_overview(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<AdminMultiSigOverviewResponse>, AuthError> {
    Ok(Json(governance::get_multisig_overview(&state).await?))
}

pub async fn get_multisig_proposal(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Path(proposal_id): Path<String>,
) -> Result<Json<AdminMultiSigProposalResponse>, AuthError> {
    Ok(Json(
        governance::get_multisig_proposal(&state, &proposal_id).await?,
    ))
}

pub async fn get_multisig_proposal_signature(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Path((proposal_id, signer)): Path<(String, String)>,
) -> Result<Json<AdminMultiSigProposalSignatureResponse>, AuthError> {
    Ok(Json(
        governance::get_multisig_proposal_signature(&state, &proposal_id, &signer).await?,
    ))
}

pub async fn propose_multisig_transaction(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminMultiSigProposalRequest>,
) -> Result<Json<AdminMultiSigProposalWriteResponse>, AuthError> {
    Ok(Json(
        governance::propose_multisig_transaction(&state, payload).await?,
    ))
}

pub async fn propose_add_multisig_signer(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminMultiSigSignerWriteRequest>,
) -> Result<Json<AdminMultiSigProposalWriteResponse>, AuthError> {
    Ok(Json(
        governance::propose_multisig_add_signer(&state, payload).await?,
    ))
}

pub async fn propose_remove_multisig_signer(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminMultiSigSignerWriteRequest>,
) -> Result<Json<AdminMultiSigProposalWriteResponse>, AuthError> {
    Ok(Json(
        governance::propose_multisig_remove_signer(&state, payload).await?,
    ))
}

pub async fn propose_update_multisig_quorum(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminMultiSigQuorumWriteRequest>,
) -> Result<Json<AdminMultiSigProposalWriteResponse>, AuthError> {
    Ok(Json(
        governance::propose_multisig_update_quorum(&state, payload).await?,
    ))
}

pub async fn sign_multisig_proposal(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Path(proposal_id): Path<String>,
) -> Result<Json<AdminMultiSigProposalWriteResponse>, AuthError> {
    Ok(Json(
        governance::sign_multisig_proposal(&state, &proposal_id).await?,
    ))
}

pub async fn execute_multisig_proposal(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Path(proposal_id): Path<String>,
) -> Result<Json<AdminMultiSigProposalWriteResponse>, AuthError> {
    Ok(Json(
        governance::execute_multisig_proposal(&state, &proposal_id).await?,
    ))
}

pub async fn cancel_multisig_proposal(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Path(proposal_id): Path<String>,
) -> Result<Json<AdminMultiSigProposalWriteResponse>, AuthError> {
    Ok(Json(
        governance::cancel_multisig_proposal(&state, &proposal_id).await?,
    ))
}
