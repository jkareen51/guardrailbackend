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
