use axum::{Router, routing::get};

use crate::{
    app::AppState,
    module::market::controller::{
        get_payment_token_quote, get_supported_market_currencies,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route(
            "/market/quotes/payment-token",
            get(get_payment_token_quote),
        )
        .route(
            "/market/quotes/ngn-payment-token",
            get(get_payment_token_quote),
        )
        .route(
            "/market/supported-currencies",
            get(get_supported_market_currencies),
        )
}
