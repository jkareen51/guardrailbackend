use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};

use crate::{
    app::AppState,
    module::{
        asset_request::schema::{
            AdminUpdateAssetRequestStatusRequest, AssetRequestDeployResponse,
            AssetRequestListResponse, AssetRequestResponse, CreateAssetRequestRequest,
            ListAssetRequestsQuery,
        },
        auth::error::AuthError,
    },
    service::{asset_request, jwt::AuthenticatedUser},
};

pub async fn create_asset_request(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateAssetRequestRequest>,
) -> Result<Json<AssetRequestResponse>, AuthError> {
    Ok(Json(
        asset_request::create_asset_request(&state, authenticated_user.user_id, payload).await?,
    ))
}

pub async fn list_my_asset_requests(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Query(query): Query<ListAssetRequestsQuery>,
) -> Result<Json<AssetRequestListResponse>, AuthError> {
    Ok(Json(
        asset_request::list_my_asset_requests(&state, authenticated_user.user_id, query).await?,
    ))
}

pub async fn get_asset_request(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(request_id): Path<String>,
) -> Result<Json<AssetRequestResponse>, AuthError> {
    Ok(Json(
        asset_request::get_asset_request(&state, authenticated_user.user_id, &request_id).await?,
    ))
}

pub async fn list_asset_requests_for_review(
    State(state): State<AppState>,
    Query(query): Query<ListAssetRequestsQuery>,
) -> Result<Json<AssetRequestListResponse>, AuthError> {
    Ok(Json(
        asset_request::list_asset_requests(&state, query).await?,
    ))
}

pub async fn update_asset_request_status(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(request_id): Path<String>,
    Json(payload): Json<AdminUpdateAssetRequestStatusRequest>,
) -> Result<Json<AssetRequestResponse>, AuthError> {
    Ok(Json(
        asset_request::update_asset_request_status(
            &state,
            authenticated_user.user_id,
            &request_id,
            payload,
        )
        .await?,
    ))
}

pub async fn deploy_asset_request(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(request_id): Path<String>,
) -> Result<Json<AssetRequestDeployResponse>, AuthError> {
    Ok(Json(
        asset_request::deploy_asset_request(&state, authenticated_user.user_id, &request_id)
            .await?,
    ))
}
