use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};

use crate::{
    app::AppState,
    middleware::admin::require_admin,
    module::admin::controller::{
        cancel_multisig_proposal, execute_multisig_proposal, get_access_control_overview,
        get_access_control_role_membership, get_multisig_overview, get_multisig_proposal,
        get_multisig_proposal_signature, grant_access_control_role, me,
        propose_add_multisig_signer, propose_multisig_transaction, propose_remove_multisig_signer,
        propose_update_multisig_quorum, revoke_access_control_role, sign_multisig_proposal,
        upload_image, wallet_challenge, wallet_connect,
    },
};

pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        .route("/me", get(me))
        .route("/uploads/images", post(upload_image))
        .route(
            "/contracts/access-control",
            get(get_access_control_overview),
        )
        .route(
            "/contracts/access-control/roles/{role}/accounts/{account}",
            get(get_access_control_role_membership),
        )
        .route(
            "/contracts/access-control/grant",
            post(grant_access_control_role),
        )
        .route(
            "/contracts/access-control/revoke",
            post(revoke_access_control_role),
        )
        .route("/contracts/multisig", get(get_multisig_overview))
        .route(
            "/contracts/multisig/proposals",
            post(propose_multisig_transaction),
        )
        .route(
            "/contracts/multisig/signers/add",
            post(propose_add_multisig_signer),
        )
        .route(
            "/contracts/multisig/signers/remove",
            post(propose_remove_multisig_signer),
        )
        .route(
            "/contracts/multisig/quorum",
            post(propose_update_multisig_quorum),
        )
        .route(
            "/contracts/multisig/proposals/{proposal_id}",
            get(get_multisig_proposal),
        )
        .route(
            "/contracts/multisig/proposals/{proposal_id}/signers/{signer}",
            get(get_multisig_proposal_signature),
        )
        .route(
            "/contracts/multisig/proposals/{proposal_id}/sign",
            post(sign_multisig_proposal),
        )
        .route(
            "/contracts/multisig/proposals/{proposal_id}/execute",
            post(execute_multisig_proposal),
        )
        .route(
            "/contracts/multisig/proposals/{proposal_id}/cancel",
            post(cancel_multisig_proposal),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            require_admin,
        ));

    Router::new()
        .route("/auth/wallet/challenge", post(wallet_challenge))
        .route("/auth/wallet/connect", post(wallet_connect))
        .merge(protected_routes)
}
