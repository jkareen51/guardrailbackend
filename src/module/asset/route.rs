use axum::{
    Router, middleware as axum_middleware,
    routing::{delete, get, post, put},
};

use crate::{
    app::AppState,
    middleware::{admin::require_admin, user::require_auth},
    module::asset::controller::{
        approve_payment_token, archive_asset, burn_asset, cancel_redemption, check_transfer,
        claim_yield, controller_transfer, create_asset, disable_controller, get_asset,
        get_asset_by_proposal, get_asset_by_slug, get_asset_detail, get_asset_detail_by_proposal,
        get_asset_detail_by_slug, get_asset_history, get_asset_history_by_proposal,
        get_asset_history_by_slug, get_asset_holder_state, get_asset_type, get_factory_status,
        issue_asset, list_asset_types, list_assets, list_assets_by_type, pause_factory,
        preview_purchase, preview_redemption, process_redemption, purchase_asset, redeem_asset,
        register_asset_type, set_asset_catalog, set_asset_state, set_compliance_registry,
        set_metadata_hash, set_pricing, set_redemption_price, set_self_service_purchase_enabled,
        set_subscription_price, set_treasury, unpause_factory, unregister_asset_type,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/assets/factory", get(get_factory_status))
        .route("/assets/types", get(list_asset_types))
        .route("/assets/types/{asset_type_id}", get(get_asset_type))
        .route("/assets", get(list_assets))
        .route("/assets/by-type/{asset_type_id}", get(list_assets_by_type))
        .route(
            "/assets/proposals/{proposal_id}/history",
            get(get_asset_history_by_proposal),
        )
        .route(
            "/assets/proposals/{proposal_id}/detail",
            get(get_asset_detail_by_proposal),
        )
        .route(
            "/assets/proposals/{proposal_id}",
            get(get_asset_by_proposal),
        )
        .route(
            "/assets/slug/{slug}/history",
            get(get_asset_history_by_slug),
        )
        .route("/assets/slug/{slug}/detail", get(get_asset_detail_by_slug))
        .route("/assets/slug/{slug}", get(get_asset_by_slug))
        .route("/assets/{asset_address}/history", get(get_asset_history))
        .route("/assets/{asset_address}/detail", get(get_asset_detail))
        .route("/assets/{asset_address}", get(get_asset))
        .route(
            "/assets/{asset_address}/holders/{wallet_address}",
            get(get_asset_holder_state),
        )
        .route(
            "/assets/{asset_address}/preview/purchase",
            post(preview_purchase),
        )
        .route(
            "/assets/{asset_address}/preview/redemption",
            post(preview_redemption),
        )
        .route(
            "/assets/{asset_address}/check/transfer",
            post(check_transfer),
        )
}

pub fn admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/assets/types", post(register_asset_type))
        .route(
            "/assets/types/{asset_type_id}",
            delete(unregister_asset_type),
        )
        .route("/assets/factory/pause", post(pause_factory))
        .route("/assets/factory/unpause", post(unpause_factory))
        .route("/assets", post(create_asset))
        .route("/assets/{asset_address}/issue", post(issue_asset))
        .route("/assets/{asset_address}/burn", post(burn_asset))
        .route("/assets/{asset_address}/archive", post(archive_asset))
        .route("/assets/{asset_address}/state", put(set_asset_state))
        .route(
            "/assets/{asset_address}/subscription-price",
            put(set_subscription_price),
        )
        .route(
            "/assets/{asset_address}/redemption-price",
            put(set_redemption_price),
        )
        .route("/assets/{asset_address}/pricing", put(set_pricing))
        .route(
            "/assets/{asset_address}/self-service-purchase",
            put(set_self_service_purchase_enabled),
        )
        .route("/assets/{asset_address}/metadata", put(set_metadata_hash))
        .route("/assets/{asset_address}/catalog", put(set_asset_catalog))
        .route(
            "/assets/{asset_address}/compliance-registry",
            put(set_compliance_registry),
        )
        .route("/assets/{asset_address}/treasury", put(set_treasury))
        .route(
            "/assets/{asset_address}/controller/disable",
            post(disable_controller),
        )
        .route(
            "/assets/{asset_address}/controller/transfer",
            post(controller_transfer),
        )
        .route(
            "/assets/{asset_address}/redemptions/process",
            post(process_redemption),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_admin))
}

pub fn user_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/assets/{asset_address}/payment-token/approve",
            post(approve_payment_token),
        )
        .route("/assets/{asset_address}/purchase", post(purchase_asset))
        .route("/assets/{asset_address}/yield/claim", post(claim_yield))
        .route("/assets/{asset_address}/redeem", post(redeem_asset))
        .route(
            "/assets/{asset_address}/redemptions/cancel",
            post(cancel_redemption),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_auth))
}
