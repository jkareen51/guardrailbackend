use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::{
    app::AppState,
    module::{
        asset::schema::{
            AdminBurnAssetRequest, AdminControllerTransferRequest, AdminCreateAssetRequest,
            AdminIssueAssetRequest, AdminProcessRedemptionRequest, AdminRegisterAssetTypeRequest,
            AdminSetAssetComplianceRegistryRequest, AdminSetAssetMetadataRequest,
            AdminSetAssetPriceRequest, AdminSetAssetPricingRequest,
            AdminSetAssetSelfServicePurchaseRequest, AdminSetAssetStateRequest,
            AdminSetAssetTreasuryRequest, AssetFactoryStatusResponse, AssetFactoryWriteResponse,
            AssetHolderStateResponse, AssetListResponse, AssetPreviewRequest, AssetPreviewResponse,
            AssetResponse, AssetTransferCheckResponse, AssetTypeListResponse, AssetTypeResponse,
            AssetTypeWriteResponse, AssetWriteResponse, GaslessApprovePaymentTokenRequest,
            GaslessAssetActionResponse, GaslessCancelRedemptionRequest, GaslessClaimYieldRequest,
            GaslessPurchaseAssetRequest, GaslessRedeemAssetRequest,
        },
        auth::error::AuthError,
    },
    service::{asset, jwt::AuthenticatedUser},
};

pub async fn get_factory_status(
    State(state): State<AppState>,
) -> Result<Json<AssetFactoryStatusResponse>, AuthError> {
    Ok(Json(asset::get_factory_status(&state).await?))
}

pub async fn list_asset_types(
    State(state): State<AppState>,
) -> Result<Json<AssetTypeListResponse>, AuthError> {
    Ok(Json(asset::list_asset_types(&state).await?))
}

pub async fn get_asset_type(
    State(state): State<AppState>,
    Path(asset_type_id): Path<String>,
) -> Result<Json<AssetTypeResponse>, AuthError> {
    Ok(Json(asset::get_asset_type(&state, &asset_type_id).await?))
}

pub async fn list_assets(
    State(state): State<AppState>,
) -> Result<Json<AssetListResponse>, AuthError> {
    Ok(Json(asset::list_assets(&state).await?))
}

pub async fn list_assets_by_type(
    State(state): State<AppState>,
    Path(asset_type_id): Path<String>,
) -> Result<Json<AssetListResponse>, AuthError> {
    Ok(Json(
        asset::list_assets_by_type(&state, &asset_type_id).await?,
    ))
}

pub async fn get_asset_by_proposal(
    State(state): State<AppState>,
    Path(proposal_id): Path<String>,
) -> Result<Json<AssetResponse>, AuthError> {
    Ok(Json(
        asset::get_asset_by_proposal(&state, &proposal_id).await?,
    ))
}

pub async fn get_asset(
    State(state): State<AppState>,
    Path(asset_address): Path<String>,
) -> Result<Json<AssetResponse>, AuthError> {
    Ok(Json(asset::get_asset(&state, &asset_address).await?))
}

pub async fn get_asset_holder_state(
    State(state): State<AppState>,
    Path((asset_address, wallet_address)): Path<(String, String)>,
) -> Result<Json<AssetHolderStateResponse>, AuthError> {
    Ok(Json(
        asset::get_asset_holder_state(&state, &asset_address, &wallet_address).await?,
    ))
}

pub async fn preview_purchase(
    State(state): State<AppState>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AssetPreviewRequest>,
) -> Result<Json<AssetPreviewResponse>, AuthError> {
    Ok(Json(
        asset::preview_purchase(&state, &asset_address, payload).await?,
    ))
}

pub async fn preview_redemption(
    State(state): State<AppState>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AssetPreviewRequest>,
) -> Result<Json<AssetPreviewResponse>, AuthError> {
    Ok(Json(
        asset::preview_redemption(&state, &asset_address, payload).await?,
    ))
}

pub async fn check_transfer(
    State(state): State<AppState>,
    Path(asset_address): Path<String>,
    Json(payload): Json<crate::module::asset::schema::AssetCheckTransferRequest>,
) -> Result<Json<AssetTransferCheckResponse>, AuthError> {
    Ok(Json(
        asset::check_transfer(&state, &asset_address, payload).await?,
    ))
}

pub async fn register_asset_type(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminRegisterAssetTypeRequest>,
) -> Result<Json<AssetTypeWriteResponse>, AuthError> {
    Ok(Json(
        asset::register_asset_type(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn unregister_asset_type(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_type_id): Path<String>,
) -> Result<Json<AssetTypeWriteResponse>, AuthError> {
    Ok(Json(
        asset::unregister_asset_type(&state, authenticated_user.user_id, &asset_type_id).await?,
    ))
}

pub async fn pause_factory(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<AssetFactoryWriteResponse>, AuthError> {
    Ok(Json(
        asset::pause_factory(&state, authenticated_user.user_id).await?,
    ))
}

pub async fn unpause_factory(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<AssetFactoryWriteResponse>, AuthError> {
    Ok(Json(
        asset::unpause_factory(&state, authenticated_user.user_id).await?,
    ))
}

pub async fn create_asset(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<AdminCreateAssetRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::create_asset(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn issue_asset(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminIssueAssetRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::issue_asset(&state, authenticated_user.user_id, &asset_address, payload).await?,
    ))
}

pub async fn burn_asset(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminBurnAssetRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::burn_asset(&state, authenticated_user.user_id, &asset_address, payload).await?,
    ))
}

pub async fn set_asset_state(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminSetAssetStateRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::set_asset_state(&state, authenticated_user.user_id, &asset_address, payload).await?,
    ))
}

pub async fn set_subscription_price(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminSetAssetPriceRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::set_subscription_price(&state, authenticated_user.user_id, &asset_address, payload)
            .await?,
    ))
}

pub async fn set_redemption_price(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminSetAssetPriceRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::set_redemption_price(&state, authenticated_user.user_id, &asset_address, payload)
            .await?,
    ))
}

pub async fn set_pricing(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminSetAssetPricingRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::set_pricing(&state, authenticated_user.user_id, &asset_address, payload).await?,
    ))
}

pub async fn set_self_service_purchase_enabled(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminSetAssetSelfServicePurchaseRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::set_self_service_purchase_enabled(
            &state,
            authenticated_user.user_id,
            &asset_address,
            payload,
        )
        .await?,
    ))
}

pub async fn set_metadata_hash(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminSetAssetMetadataRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::set_metadata_hash(&state, authenticated_user.user_id, &asset_address, payload)
            .await?,
    ))
}

pub async fn set_compliance_registry(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminSetAssetComplianceRegistryRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::set_compliance_registry(&state, authenticated_user.user_id, &asset_address, payload)
            .await?,
    ))
}

pub async fn set_treasury(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminSetAssetTreasuryRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::set_treasury(&state, authenticated_user.user_id, &asset_address, payload).await?,
    ))
}

pub async fn disable_controller(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::disable_controller(&state, authenticated_user.user_id, &asset_address).await?,
    ))
}

pub async fn controller_transfer(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminControllerTransferRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::controller_transfer(&state, authenticated_user.user_id, &asset_address, payload)
            .await?,
    ))
}

pub async fn process_redemption(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<AdminProcessRedemptionRequest>,
) -> Result<Json<AssetWriteResponse>, AuthError> {
    Ok(Json(
        asset::process_redemption(&state, authenticated_user.user_id, &asset_address, payload)
            .await?,
    ))
}

pub async fn approve_payment_token(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<GaslessApprovePaymentTokenRequest>,
) -> Result<Json<GaslessAssetActionResponse>, AuthError> {
    Ok(Json(
        asset::approve_payment_token(&state, authenticated_user.user_id, &asset_address, payload)
            .await?,
    ))
}

pub async fn purchase_asset(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<GaslessPurchaseAssetRequest>,
) -> Result<Json<GaslessAssetActionResponse>, AuthError> {
    Ok(Json(
        asset::purchase_asset(&state, authenticated_user.user_id, &asset_address, payload).await?,
    ))
}

pub async fn claim_yield(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<GaslessClaimYieldRequest>,
) -> Result<Json<GaslessAssetActionResponse>, AuthError> {
    Ok(Json(
        asset::claim_yield(&state, authenticated_user.user_id, &asset_address, payload).await?,
    ))
}

pub async fn redeem_asset(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<GaslessRedeemAssetRequest>,
) -> Result<Json<GaslessAssetActionResponse>, AuthError> {
    Ok(Json(
        asset::redeem_asset(&state, authenticated_user.user_id, &asset_address, payload).await?,
    ))
}

pub async fn cancel_redemption(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(asset_address): Path<String>,
    Json(payload): Json<GaslessCancelRedemptionRequest>,
) -> Result<Json<GaslessAssetActionResponse>, AuthError> {
    Ok(Json(
        asset::cancel_redemption(&state, authenticated_user.user_id, &asset_address, payload)
            .await?,
    ))
}
