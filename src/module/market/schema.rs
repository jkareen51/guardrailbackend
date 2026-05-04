use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

fn default_market_currency() -> String {
    "ngn".to_owned()
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PaymentTokenQuoteQuery {
    #[serde(default = "default_market_currency")]
    pub market_currency: String,
    #[serde(alias = "amount_ngn")]
    pub amount: Option<String>,
    #[serde(alias = "subscription_price_ngn")]
    pub subscription_price: Option<String>,
    #[serde(alias = "redemption_price_ngn")]
    pub redemption_price: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentTokenQuoteResponse {
    pub market_currency: String,
    pub payment_token_coin_id: String,
    pub payment_token_address: String,
    pub payment_token_symbol: String,
    pub payment_token_decimals: u8,
    pub market_currency_per_payment_token: String,
    pub usd_per_payment_token: String,
    pub last_updated_at: Option<i64>,
    pub amount: Option<MarketAmountQuote>,
    pub subscription_price: Option<MarketAmountQuote>,
    pub redemption_price: Option<MarketAmountQuote>,
}

#[derive(Debug, Serialize)]
pub struct MarketAmountQuote {
    pub market_currency_amount: String,
    pub payment_token_amount: String,
    pub payment_token_base_units: String,
}

#[derive(Debug, Serialize)]
pub struct SupportedMarketCurrenciesResponse {
    pub supported_currencies: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SearchMarketsQuery {
    pub q: Option<String>,
    pub category_slug: Option<String>,
    pub subcategory_slug: Option<String>,
    pub tag_slug: Option<String>,
    pub trading_status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PublicEventTeaserResponse {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub featured: bool,
    pub breaking: bool,
    pub neg_risk: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketCurrentPricesResponse {
    pub yes_bps: i32,
    pub no_bps: i32,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketStatsResponse {
    pub volume_usd: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketQuoteSummaryResponse {
    pub buy_yes_bps: i32,
    pub buy_no_bps: i32,
    pub as_of: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct PublicMarketCardResponse {
    pub id: String,
    pub slug: String,
    pub label: String,
    pub question: String,
    pub question_id: String,
    pub condition_id: Option<String>,
    pub market_type: String,
    pub outcomes: Vec<String>,
    pub end_time: DateTime<Utc>,
    pub sort_order: i32,
    pub trading_status: String,
    pub current_prices: Option<MarketCurrentPricesResponse>,
    pub stats: Option<MarketStatsResponse>,
    pub quote_summary: Option<MarketQuoteSummaryResponse>,
    pub event: PublicEventTeaserResponse,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketListResponse {
    pub markets: Vec<PublicMarketCardResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct TagSummaryResponse {
    pub slug: String,
    pub label: String,
    pub event_count: i64,
    pub market_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct TagsResponse {
    pub tags: Vec<TagSummaryResponse>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventResponse {
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub rules: String,
    pub context: Option<String>,
    pub additional_context: Option<String>,
    pub resolution_sources: Vec<String>,
    pub resolution_timezone: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub sort_at: Option<DateTime<Utc>>,
    pub featured: bool,
    pub breaking: bool,
    pub searchable: bool,
    pub visible: bool,
    pub hide_resolved_by_default: bool,
    pub publication_status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventOnChainResponse {
    pub event_id: String,
    pub group_id: String,
    pub series_id: String,
    pub neg_risk: bool,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketResponse {
    pub id: String,
    pub slug: String,
    pub label: String,
    pub question: String,
    pub question_id: String,
    pub condition_id: Option<String>,
    pub market_type: String,
    pub outcomes: Vec<String>,
    pub end_time: DateTime<Utc>,
    pub sort_order: i32,
    pub publication_status: String,
    pub trading_status: String,
    pub current_prices: Option<MarketCurrentPricesResponse>,
    pub stats: Option<MarketStatsResponse>,
    pub quote_summary: Option<MarketQuoteSummaryResponse>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PositionOutcomeResponse {
    pub outcome_index: i32,
    pub outcome_label: String,
    pub token_amount: String,
    pub estimated_value_usdc: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PortfolioSummaryResponse {
    pub cash_balance: String,
    pub portfolio_balance: String,
    pub total_balance: String,
    pub total_buy_amount: String,
    pub total_sell_amount: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct PortfolioMarketSummaryResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub buy_amount: String,
    pub sell_amount: String,
    pub portfolio_balance: String,
    pub positions: Vec<PositionOutcomeResponse>,
    pub last_traded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PortfolioTradeHistoryItemResponse {
    pub id: String,
    pub execution_source: String,
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub action: String,
    pub outcome_index: i32,
    pub outcome_label: String,
    pub usdc_amount: String,
    pub token_amount: String,
    pub price_bps: i32,
    pub price: f64,
    pub tx_hash: Option<String>,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MyPortfolioResponse {
    pub wallet_address: String,
    pub account_kind: String,
    pub summary: PortfolioSummaryResponse,
    pub markets: Vec<PortfolioMarketSummaryResponse>,
    pub history: Vec<PortfolioTradeHistoryItemResponse>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ListEventsQuery {
    pub category_slug: Option<String>,
    pub subcategory_slug: Option<String>,
    pub tag_slug: Option<String>,
    pub featured: Option<bool>,
    pub breaking: Option<bool>,
    pub include_markets: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MarketsHomeQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CategorySummaryResponse {
    pub slug: String,
    pub label: String,
    pub event_count: i64,
    pub market_count: i64,
    pub featured_event_count: i64,
    pub breaking_event_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct CategoriesResponse {
    pub categories: Vec<CategorySummaryResponse>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PublicEventCardResponse {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub featured: bool,
    pub breaking: bool,
    pub neg_risk: bool,
    pub starts_at: Option<DateTime<Utc>>,
    pub sort_at: Option<DateTime<Utc>>,
    pub market_count: i64,
    pub markets: Option<Vec<PublicMarketCardResponse>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventListResponse {
    pub events: Vec<PublicEventCardResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketsHomeResponse {
    pub featured: Vec<PublicMarketCardResponse>,
    pub breaking: Vec<PublicMarketCardResponse>,
    pub newest: Vec<PublicMarketCardResponse>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventDetailResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub markets_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventMarketsResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub markets: Vec<MarketResponse>,
}
