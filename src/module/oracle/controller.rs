use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        oracle::schema::{
            AdminAnchorDocumentRequest, AdminSetTrustedOracleRequest,
            AdminSubmitValuationAndSyncPricingRequest, AdminSubmitValuationRequest,
            OracleDocumentResponse, OracleDocumentWriteResponse, OracleTrustedOracleResponse,
            OracleTrustedOracleWriteResponse, OracleValuationResponse,
            OracleValuationWriteResponse,
        },
    },
    service::{jwt::AuthenticatedUser, oracle},
};

pub async fn get_trusted_oracle(
    State(state): State<AppState>,
    Path(oracle_address): Path<String>,
) -> Result<Json<OracleTrustedOracleResponse>, AuthError> {
    Ok(Json(
        oracle::get_trusted_oracle(&state, &oracle_address).await?,
    ))
}

pub async fn get_valuation(
    State(state): State<AppState>,
    Path(asset_address): Path<String>,
) -> Result<Json<OracleValuationResponse>, AuthError> {
    Ok(Json(oracle::get_valuation(&state, &asset_address).await?))
}

pub async fn get_document(
    State(state): State<AppState>,
    Path((asset_address, document_type)): Path<(String, String)>,
) -> Result<Json<OracleDocumentResponse>, AuthError> {
    Ok(Json(
        oracle::get_document(&state, &asset_address, &document_type).await?,
    ))
}

pub async fn set_trusted_oracle(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(oracle_address): Path<String>,
    Json(payload): Json<AdminSetTrustedOracleRequest>,
) -> Result<Json<OracleTrustedOracleWriteResponse>, AuthError> {
    Ok(Json(
        oracle::set_trusted_oracle(&state, authenticated_user.user_id, &oracle_address, payload)
            .await?,
    ))
}

pub async fn submit_valuation(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminSubmitValuationRequest>,
) -> Result<Json<OracleValuationWriteResponse>, AuthError> {
    Ok(Json(
        oracle::submit_valuation(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn submit_valuation_and_sync_pricing(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminSubmitValuationAndSyncPricingRequest>,
) -> Result<Json<OracleValuationWriteResponse>, AuthError> {
    Ok(Json(
        oracle::submit_valuation_and_sync_pricing(&state, authenticated_user.user_id, payload)
            .await?,
    ))
}

pub async fn anchor_document(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path((asset_address, document_type)): Path<(String, String)>,
    Json(payload): Json<AdminAnchorDocumentRequest>,
) -> Result<Json<OracleDocumentWriteResponse>, AuthError> {
    Ok(Json(
        oracle::anchor_document(
            &state,
            authenticated_user.user_id,
            &asset_address,
            &document_type,
            payload,
        )
        .await?,
    ))
}
