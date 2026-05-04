use axum::{
    Extension, Json,
    extract::{Query, State},
};

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::schema::{
            CategoriesResponse, EventDetailResponse, EventListResponse, EventMarketsResponse,
            ListEventsQuery, MarketListResponse, MarketsHomeQuery, MarketsHomeResponse,
            MyPortfolioResponse, PaymentTokenQuoteQuery, PaymentTokenQuoteResponse,
            SearchMarketsQuery, SupportedMarketCurrenciesResponse, TagsResponse,
        },
    },
    service::{browser, jwt::AuthenticatedUser, market},
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

pub async fn search_markets(
    State(state): State<AppState>,
    Query(query): Query<SearchMarketsQuery>,
) -> Result<Json<MarketListResponse>, AuthError> {
    Ok(Json(browser::search_markets(&state, query).await?))
}

pub async fn list_tags(State(state): State<AppState>) -> Result<Json<TagsResponse>, AuthError> {
    Ok(Json(browser::list_tags(&state).await?))
}

pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<CategoriesResponse>, AuthError> {
    Ok(Json(browser::list_categories(&state).await?))
}

pub async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<EventListResponse>, AuthError> {
    Ok(Json(browser::list_events(&state, query).await?))
}

pub async fn fetch_event(
    State(state): State<AppState>,
    axum::extract::Path(event_id): axum::extract::Path<String>,
) -> Result<Json<EventDetailResponse>, AuthError> {
    Ok(Json(browser::fetch_event(&state, &event_id).await?))
}

pub async fn fetch_event_markets(
    State(state): State<AppState>,
    axum::extract::Path(event_id): axum::extract::Path<String>,
) -> Result<Json<EventMarketsResponse>, AuthError> {
    Ok(Json(browser::fetch_event_markets(&state, &event_id).await?))
}

pub async fn fetch_markets_home(
    State(state): State<AppState>,
    Query(query): Query<MarketsHomeQuery>,
) -> Result<Json<MarketsHomeResponse>, AuthError> {
    Ok(Json(browser::fetch_markets_home(&state, query).await?))
}

pub async fn get_my_portfolio(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<MyPortfolioResponse>, AuthError> {
    Ok(Json(
        browser::get_my_portfolio(&state, authenticated_user.user_id).await?,
    ))
}
