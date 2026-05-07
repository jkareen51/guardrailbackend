use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post, put},
};

use crate::{
    app::AppState,
    middleware::{admin::require_admin, user::require_auth},
    module::asset_request::controller::{
        create_asset_request, deploy_asset_request, get_asset_request,
        list_asset_requests_for_review, list_my_asset_requests, update_asset_request_status,
    },
};

pub fn user_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/asset-requests", post(create_asset_request))
        .route("/asset-requests/me", get(list_my_asset_requests))
        .route("/asset-requests/{request_id}", get(get_asset_request))
        .route_layer(axum_middleware::from_fn_with_state(state, require_auth))
}

pub fn admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/asset-requests", get(list_asset_requests_for_review))
        .route(
            "/asset-requests/{request_id}/status",
            put(update_asset_request_status),
        )
        .route(
            "/asset-requests/{request_id}/deploy",
            post(deploy_asset_request),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_admin))
}
