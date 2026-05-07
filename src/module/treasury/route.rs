use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};

use crate::{
    app::AppState,
    middleware::admin::require_admin,
    module::treasury::controller::{
        approve_payment_token, deposit_asset_liquidity, deposit_yield, emergency_withdraw,
        get_treasury_asset, get_treasury_status, pause_treasury, register_asset_token,
        release_capital, unpause_treasury,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/treasury", get(get_treasury_status))
        .route("/treasury/assets/{asset_address}", get(get_treasury_asset))
}

pub fn admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/treasury/payment-token/approve",
            post(approve_payment_token),
        )
        .route("/treasury/liquidity/deposit", post(deposit_asset_liquidity))
        .route("/treasury/capital/release", post(release_capital))
        .route("/treasury/yield/deposit", post(deposit_yield))
        .route("/treasury/emergency-withdraw", post(emergency_withdraw))
        .route("/treasury/pause", post(pause_treasury))
        .route("/treasury/unpause", post(unpause_treasury))
        .route(
            "/treasury/assets/{asset_address}/register-token",
            post(register_asset_token),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_admin))
}
