use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post, put},
};

use crate::{
    app::AppState,
    middleware::admin::require_admin,
    module::oracle::controller::{
        anchor_document, get_document, get_trusted_oracle, get_valuation, set_trusted_oracle,
        submit_valuation, submit_valuation_and_sync_pricing,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route(
            "/oracle/trusted-oracles/{oracle_address}",
            get(get_trusted_oracle),
        )
        .route(
            "/oracle/assets/{asset_address}/valuation",
            get(get_valuation),
        )
        .route(
            "/oracle/assets/{asset_address}/documents/{document_type}",
            get(get_document),
        )
}

pub fn admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/oracle/trusted-oracles/{oracle_address}",
            put(set_trusted_oracle),
        )
        .route("/oracle/valuations", post(submit_valuation))
        .route(
            "/oracle/valuations/sync-pricing",
            post(submit_valuation_and_sync_pricing),
        )
        .route(
            "/oracle/assets/{asset_address}/documents/{document_type}",
            put(anchor_document),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_admin))
}
