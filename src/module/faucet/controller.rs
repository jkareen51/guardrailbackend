use axum::{
    Json,
    extract::{Extension, Query, State},
};

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        faucet::schema::{
            FaucetUsdcBalanceQuery, FaucetUsdcBalanceResponse, FaucetUsdcRequest,
            FaucetUsdcResponse,
        },
    },
    service::{faucet, jwt::AuthenticatedUser},
};

pub async fn faucet_usdc(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    payload: Option<Json<FaucetUsdcRequest>>,
) -> Result<Json<FaucetUsdcResponse>, AuthError> {
    let requested_amount = payload.and_then(|Json(value)| value.amount);

    Ok(Json(
        faucet::request_usdc_faucet(
            &state,
            authenticated_user.user_id,
            requested_amount.as_deref(),
        )
        .await?,
    ))
}

pub async fn mock_usdc_balance(
    State(state): State<AppState>,
    Query(query): Query<FaucetUsdcBalanceQuery>,
) -> Result<Json<FaucetUsdcBalanceResponse>, AuthError> {
    Ok(Json(get_balance_response(&state, &query.address).await?))
}

async fn get_balance_response(
    state: &AppState,
    address: &str,
) -> Result<FaucetUsdcBalanceResponse, AuthError> {
    faucet::get_mock_usdc_balance(state, address).await
}
