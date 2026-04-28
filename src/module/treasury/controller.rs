use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        treasury::schema::{
            AdminApproveTreasuryPaymentTokenRequest, AdminDepositAssetLiquidityRequest,
            AdminDepositYieldRequest, AdminEmergencyWithdrawRequest, AdminReleaseCapitalRequest,
            TreasuryAssetResponse, TreasuryAssetWriteResponse,
            TreasuryPaymentTokenApprovalResponse, TreasuryStatusResponse,
            TreasuryStatusWriteResponse,
        },
    },
    service::{jwt::AuthenticatedUser, treasury},
};

pub async fn get_treasury_status(
    State(state): State<AppState>,
) -> Result<Json<TreasuryStatusResponse>, AuthError> {
    Ok(Json(treasury::get_treasury_status(&state).await?))
}

pub async fn get_treasury_asset(
    State(state): State<AppState>,
    Path(asset_address): Path<String>,
) -> Result<Json<TreasuryAssetResponse>, AuthError> {
    Ok(Json(
        treasury::get_treasury_asset(&state, &asset_address).await?,
    ))
}

pub async fn approve_payment_token(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminApproveTreasuryPaymentTokenRequest>,
) -> Result<Json<TreasuryPaymentTokenApprovalResponse>, AuthError> {
    Ok(Json(
        treasury::approve_payment_token(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn deposit_asset_liquidity(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminDepositAssetLiquidityRequest>,
) -> Result<Json<TreasuryAssetWriteResponse>, AuthError> {
    Ok(Json(
        treasury::deposit_asset_liquidity(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn release_capital(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminReleaseCapitalRequest>,
) -> Result<Json<TreasuryAssetWriteResponse>, AuthError> {
    Ok(Json(
        treasury::release_capital(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn deposit_yield(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminDepositYieldRequest>,
) -> Result<Json<TreasuryAssetWriteResponse>, AuthError> {
    Ok(Json(
        treasury::deposit_yield(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn emergency_withdraw(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminEmergencyWithdrawRequest>,
) -> Result<Json<TreasuryStatusWriteResponse>, AuthError> {
    Ok(Json(
        treasury::emergency_withdraw(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn pause_treasury(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<TreasuryStatusWriteResponse>, AuthError> {
    Ok(Json(
        treasury::pause_treasury(&state, authenticated_user.user_id).await?,
    ))
}

pub async fn unpause_treasury(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<TreasuryStatusWriteResponse>, AuthError> {
    Ok(Json(
        treasury::unpause_treasury(&state, authenticated_user.user_id).await?,
    ))
}
