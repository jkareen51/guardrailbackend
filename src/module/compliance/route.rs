use axum::{
    Router, middleware as axum_middleware,
    routing::{delete, get, post, put},
};

use crate::{
    app::AppState,
    middleware::admin::require_admin,
    module::compliance::controller::{
        add_investor_to_whitelist, batch_add_investors_to_whitelist, batch_upsert_investors,
        check_redeem, check_subscribe, check_transfer, get_access_control, get_asset_rules,
        get_investor, get_jurisdiction_restriction, remove_investor_from_whitelist,
        set_access_control, set_asset_rules, set_investor_status, set_jurisdiction_restriction,
        upsert_investor,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/compliance/access-control", get(get_access_control))
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
        .route("/compliance/access-control", put(set_access_control))
        .route("/compliance/investors/{wallet}", put(upsert_investor))
        .route("/compliance/investors/batch", post(batch_upsert_investors))
        .route(
            "/compliance/investors/whitelist/batch",
            post(batch_add_investors_to_whitelist),
        )
        .route(
            "/compliance/investors/{wallet}/whitelist",
            post(add_investor_to_whitelist),
        )
        .route(
            "/compliance/investors/{wallet}/whitelist",
            delete(remove_investor_from_whitelist),
        )
        .route(
            "/compliance/investors/{wallet}/status",
            put(set_investor_status),
        )
        .route("/compliance/assets/{asset}/rules", put(set_asset_rules))
        .route(
            "/compliance/assets/{asset}/jurisdictions/{jurisdiction}",
            put(set_jurisdiction_restriction),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_admin))
}
