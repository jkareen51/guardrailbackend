use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        compliance::schema::{
            AdminBatchUpsertComplianceInvestorsRequest,
            AdminBatchWhitelistComplianceInvestorsRequest, AdminComplianceAssetRulesUpsertResponse,
            AdminComplianceInvestorBatchUpsertResponse, AdminComplianceInvestorUpsertResponse,
            AdminComplianceJurisdictionRestrictionUpsertResponse,
            AdminSetComplianceAccessControlRequest, AdminSetComplianceAssetRulesRequest,
            AdminSetComplianceInvestorStatusRequest,
            AdminSetComplianceJurisdictionRestrictionRequest, AdminUpsertComplianceInvestorRequest,
            ComplianceAccessControlResponse, ComplianceAccessControlWriteResponse,
            ComplianceAssetRulesResponse, ComplianceCheckRedeemRequest, ComplianceCheckResponse,
            ComplianceCheckSubscribeRequest, ComplianceCheckTransferRequest,
            ComplianceInvestorResponse, ComplianceJurisdictionRestrictionResponse,
        },
    },
    service::{compliance, jwt::AuthenticatedUser},
};

pub async fn upsert_investor(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(wallet): Path<String>,
    Json(payload): Json<AdminUpsertComplianceInvestorRequest>,
) -> Result<Json<AdminComplianceInvestorUpsertResponse>, AuthError> {
    Ok(Json(
        compliance::upsert_investor(&state, authenticated_user.user_id, &wallet, payload).await?,
    ))
}

pub async fn batch_upsert_investors(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminBatchUpsertComplianceInvestorsRequest>,
) -> Result<Json<AdminComplianceInvestorBatchUpsertResponse>, AuthError> {
    Ok(Json(
        compliance::batch_upsert_investors(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn batch_add_investors_to_whitelist(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminBatchWhitelistComplianceInvestorsRequest>,
) -> Result<Json<AdminComplianceInvestorBatchUpsertResponse>, AuthError> {
    Ok(Json(
        compliance::batch_add_investors_to_whitelist(&state, authenticated_user.user_id, payload)
            .await?,
    ))
}

pub async fn add_investor_to_whitelist(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(wallet): Path<String>,
) -> Result<Json<AdminComplianceInvestorUpsertResponse>, AuthError> {
    Ok(Json(
        compliance::add_investor_to_whitelist(&state, authenticated_user.user_id, &wallet).await?,
    ))
}

pub async fn remove_investor_from_whitelist(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(wallet): Path<String>,
) -> Result<Json<AdminComplianceInvestorUpsertResponse>, AuthError> {
    Ok(Json(
        compliance::remove_investor_from_whitelist(&state, authenticated_user.user_id, &wallet)
            .await?,
    ))
}

pub async fn set_investor_status(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(wallet): Path<String>,
    Json(payload): Json<AdminSetComplianceInvestorStatusRequest>,
) -> Result<Json<AdminComplianceInvestorUpsertResponse>, AuthError> {
    Ok(Json(
        compliance::set_investor_status(&state, authenticated_user.user_id, &wallet, payload)
            .await?,
    ))
}

pub async fn set_asset_rules(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset): Path<String>,
    Json(payload): Json<AdminSetComplianceAssetRulesRequest>,
) -> Result<Json<AdminComplianceAssetRulesUpsertResponse>, AuthError> {
    Ok(Json(
        compliance::set_asset_rules(&state, authenticated_user.user_id, &asset, payload).await?,
    ))
}

pub async fn set_jurisdiction_restriction(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path((asset, jurisdiction)): Path<(String, String)>,
    Json(payload): Json<AdminSetComplianceJurisdictionRestrictionRequest>,
) -> Result<Json<AdminComplianceJurisdictionRestrictionUpsertResponse>, AuthError> {
    Ok(Json(
        compliance::set_jurisdiction_restriction(
            &state,
            authenticated_user.user_id,
            &asset,
            &jurisdiction,
            payload,
        )
        .await?,
    ))
}

pub async fn get_investor(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> Result<Json<ComplianceInvestorResponse>, AuthError> {
    Ok(Json(compliance::get_investor(&state, &wallet).await?))
}

pub async fn get_asset_rules(
    State(state): State<AppState>,
    Path(asset): Path<String>,
) -> Result<Json<ComplianceAssetRulesResponse>, AuthError> {
    Ok(Json(compliance::get_asset_rules(&state, &asset).await?))
}

pub async fn get_access_control(
    State(state): State<AppState>,
) -> Result<Json<ComplianceAccessControlResponse>, AuthError> {
    Ok(Json(compliance::get_access_control(&state).await?))
}

pub async fn set_access_control(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminSetComplianceAccessControlRequest>,
) -> Result<Json<ComplianceAccessControlWriteResponse>, AuthError> {
    Ok(Json(
        compliance::set_access_control(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn get_jurisdiction_restriction(
    State(state): State<AppState>,
    Path((asset, jurisdiction)): Path<(String, String)>,
) -> Result<Json<ComplianceJurisdictionRestrictionResponse>, AuthError> {
    Ok(Json(
        compliance::get_jurisdiction_restriction(&state, &asset, &jurisdiction).await?,
    ))
}

pub async fn check_subscribe(
    State(state): State<AppState>,
    Json(payload): Json<ComplianceCheckSubscribeRequest>,
) -> Result<Json<ComplianceCheckResponse>, AuthError> {
    Ok(Json(compliance::check_subscribe(&state, payload).await?))
}

pub async fn check_transfer(
    State(state): State<AppState>,
    Json(payload): Json<ComplianceCheckTransferRequest>,
) -> Result<Json<ComplianceCheckResponse>, AuthError> {
    Ok(Json(compliance::check_transfer(&state, payload).await?))
}

pub async fn check_redeem(
    State(state): State<AppState>,
    Json(payload): Json<ComplianceCheckRedeemRequest>,
) -> Result<Json<ComplianceCheckResponse>, AuthError> {
    Ok(Json(compliance::check_redeem(&state, payload).await?))
}
