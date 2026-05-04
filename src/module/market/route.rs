use axum::{Router, middleware as axum_middleware, routing::get};

use crate::{
    app::AppState,
    middleware::user::require_auth,
    module::market::controller::{
        fetch_event, fetch_event_markets, fetch_markets_home, get_my_portfolio,
        get_payment_token_quote, get_supported_market_currencies, list_categories, list_events,
        list_tags, search_markets,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(list_categories))
        .route("/events", get(list_events))
        .route("/events/{event_id}", get(fetch_event))
        .route("/events/{event_id}/markets", get(fetch_event_markets))
        .route("/markets/search", get(search_markets))
        .route("/markets/home", get(fetch_markets_home))
        .route("/tags", get(list_tags))
        .route("/market/quotes/payment-token", get(get_payment_token_quote))
        .route(
            "/market/quotes/ngn-payment-token",
            get(get_payment_token_quote),
        )
        .route(
            "/market/supported-currencies",
            get(get_supported_market_currencies),
        )
}

pub fn me_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me/portfolio", get(get_my_portfolio))
        .route_layer(axum_middleware::from_fn_with_state(state, require_auth))
}
