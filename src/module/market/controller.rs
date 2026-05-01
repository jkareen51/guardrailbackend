use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::schema::{
            PaymentTokenQuoteQuery, PaymentTokenQuoteResponse,
            SupportedMarketCurrenciesResponse,
        },
    },
    service::market,
};

pub async fn get_payment_token_quote(
    State(state): State<AppState>,
    Query(query): Query<PaymentTokenQuoteQuery>,
) -> Result<Json<PaymentTokenQuoteResponse>, AuthError> {
    Ok(Json(market::get_payment_token_quote(&state, query).await?))
}

pub async fn get_supported_market_currencies(
    State(state): State<AppState>,
) -> Result<Json<SupportedMarketCurrenciesResponse>, AuthError> {
    Ok(Json(market::get_supported_market_currencies(&state).await?))
}
