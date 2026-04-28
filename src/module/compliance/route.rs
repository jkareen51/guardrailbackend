use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post, put},
};

use crate::{
    app::AppState,
    middleware::admin::require_admin,
    module::compliance::controller::{
        batch_upsert_investors, check_redeem, check_subscribe, check_transfer, get_asset_rules,
        get_investor, get_jurisdiction_restriction, set_asset_rules, set_jurisdiction_restriction,
        upsert_investor,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/compliance/investors/{wallet}", get(get_investor))
        .route("/compliance/assets/{asset}/rules", get(get_asset_rules))
        .route(
            "/compliance/assets/{asset}/jurisdictions/{jurisdiction}",
            get(get_jurisdiction_restriction),
        )
        .route("/compliance/check/subscribe", post(check_subscribe))
        .route("/compliance/check/transfer", post(check_transfer))
        .route("/compliance/check/redeem", post(check_redeem))
}

pub fn admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/compliance/investors/{wallet}", put(upsert_investor))
        .route("/compliance/investors/batch", post(batch_upsert_investors))
        .route("/compliance/assets/{asset}/rules", put(set_asset_rules))
        .route(
            "/compliance/assets/{asset}/jurisdictions/{jurisdiction}",
            put(set_jurisdiction_restriction),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_admin))
}
