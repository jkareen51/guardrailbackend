use std::{collections::HashMap, str::FromStr};

use anyhow::{Result, anyhow};
use ethers_contract::Contract;
use ethers_core::types::Address;
use ethers_providers::{Http, Provider};
use reqwest::StatusCode;
use rust_decimal::{Decimal, RoundingStrategy};
use serde_json::Value;

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        auth::error::AuthError,
        market::schema::{
            MarketAmountQuote, PaymentTokenQuoteQuery, PaymentTokenQuoteResponse,
            SupportedMarketCurrenciesResponse,
        },
    },
    service::{
        asset::abi::erc20_abi,
        chain::{format_address, parse_contract_address},
        rpc,
    },
};

#[derive(Debug)]
struct PaymentTokenMetadata {
    address: String,
    symbol: String,
    decimals: u8,
}

#[derive(Debug)]
struct MarketRate {
    market_currency: String,
    market_currency_per_payment_token: Decimal,
    usd_per_payment_token: Decimal,
    last_updated_at: Option<i64>,
}

pub async fn get_payment_token_quote(
    state: &AppState,
    query: PaymentTokenQuoteQuery,
) -> Result<PaymentTokenQuoteResponse, AuthError> {
    let market_currency = normalize_market_currency(&query.market_currency)?;
    let payment_token = read_payment_token_metadata(&state.env).await?;
    let coingecko = fetch_payment_token_market_rate(state, &market_currency).await?;

    Ok(PaymentTokenQuoteResponse {
        market_currency: coingecko.market_currency.clone(),
        payment_token_coin_id: state.env.coingecko_payment_token_coin_id.clone(),
        payment_token_address: payment_token.address,
        payment_token_symbol: payment_token.symbol,
        payment_token_decimals: payment_token.decimals,
        market_currency_per_payment_token: format_decimal(
            coingecko.market_currency_per_payment_token,
        ),
        usd_per_payment_token: format_decimal(coingecko.usd_per_payment_token),
        last_updated_at: coingecko.last_updated_at,
        amount: build_amount_quote(
            query.amount.as_deref(),
            coingecko.market_currency_per_payment_token,
            payment_token.decimals,
            "amount",
        )?,
        subscription_price: build_amount_quote(
            query.subscription_price.as_deref(),
            coingecko.market_currency_per_payment_token,
            payment_token.decimals,
            "subscription_price",
        )?,
        redemption_price: build_amount_quote(
            query.redemption_price.as_deref(),
            coingecko.market_currency_per_payment_token,
            payment_token.decimals,
            "redemption_price",
        )?,
    })
}

pub async fn get_supported_market_currencies(
    state: &AppState,
) -> Result<SupportedMarketCurrenciesResponse, AuthError> {
    let url = format!(
        "{}/simple/supported_vs_currencies",
        state.env.coingecko_api_base_url.trim_end_matches('/'),
    );

    let response = coingecko_request(state, state.http_client.get(url)).await?;
    let mut supported_currencies = response
        .json::<Vec<String>>()
        .await
        .map_err(|error| AuthError::internal("failed to decode CoinGecko currencies response", error))?;

    supported_currencies.sort_unstable();
    supported_currencies.dedup();

    Ok(SupportedMarketCurrenciesResponse { supported_currencies })
}

async fn fetch_payment_token_market_rate(
    state: &AppState,
    market_currency: &str,
) -> Result<MarketRate, AuthError> {
    let url = format!(
        "{}/simple/price",
        state.env.coingecko_api_base_url.trim_end_matches('/'),
    );
    let vs_currencies = if market_currency == "usd" {
        "usd".to_owned()
    } else {
        format!("{market_currency},usd")
    };

    let response = coingecko_request(
        state,
        state.http_client.get(url).query(&[
            ("ids", state.env.coingecko_payment_token_coin_id.as_str()),
            ("vs_currencies", vs_currencies.as_str()),
            ("include_last_updated_at", "true"),
            ("precision", "full"),
        ]),
    )
    .await?;

    let payload = response
        .json::<HashMap<String, Value>>()
        .await
        .map_err(|error| AuthError::internal("failed to decode CoinGecko response", error))?;

    let entry = payload.get(&state.env.coingecko_payment_token_coin_id).ok_or_else(|| {
        AuthError::internal("CoinGecko response missing payment token", "missing coin id")
    })?;
    let entry = entry.as_object().ok_or_else(|| {
        AuthError::internal("CoinGecko response had invalid payment token shape", "invalid shape")
    })?;

    let market_currency_per_payment_token = read_decimal_field(entry, market_currency)?;
    let usd_per_payment_token = if market_currency == "usd" {
        market_currency_per_payment_token
    } else {
        read_decimal_field(entry, "usd")?
    };
    let last_updated_at = entry
        .get("last_updated_at")
        .and_then(Value::as_i64);

    if market_currency_per_payment_token <= Decimal::ZERO {
        return Err(AuthError::internal(
            "CoinGecko market price must be positive",
            "non-positive market price",
        ));
    }

    Ok(MarketRate {
        market_currency: market_currency.to_owned(),
        market_currency_per_payment_token,
        usd_per_payment_token,
        last_updated_at,
    })
}

async fn coingecko_request(
    state: &AppState,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, AuthError> {
    let request = if let Some(api_key) = state.env.coingecko_demo_api_key.as_deref() {
        request.header("x-cg-demo-api-key", api_key)
    } else {
        request
    };

    let response = request
        .send()
        .await
        .map_err(|error| AuthError::internal("failed to reach CoinGecko", error))?;

    if response.status() == StatusCode::TOO_MANY_REQUESTS {
        return Err(AuthError::too_many_requests(
            "CoinGecko rate limit exceeded, retry shortly",
        ));
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::error!(%status, body, "coingecko request failed");
        return Err(AuthError::internal(
            "CoinGecko request failed",
            anyhow!("coingecko returned status {status}"),
        ));
    }

    Ok(response)
}

async fn read_payment_token_metadata(
    env: &Environment,
) -> Result<PaymentTokenMetadata, AuthError> {
    let token_address = parse_contract_address(&env.payment_token_address)
        .map_err(|error| AuthError::internal("invalid payment token address", error))?;
    let contract = read_erc20_contract(env, token_address)
        .await
        .map_err(|error| {
            AuthError::internal("failed to build payment token read contract", error)
        })?;

    let symbol = contract
        .method::<_, String>("symbol", ())
        .map_err(|error| {
            AuthError::internal("failed to build payment token symbol call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call payment token symbol", error))?;
    let decimals = contract
        .method::<_, u8>("decimals", ())
        .map_err(|error| {
            AuthError::internal("failed to build payment token decimals call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call payment token decimals", error))?;

    Ok(PaymentTokenMetadata {
        address: format_address(token_address),
        symbol,
        decimals,
    })
}

async fn read_erc20_contract(
    env: &Environment,
    token_address: Address,
) -> Result<Contract<Provider<Http>>> {
    let provider = rpc::monad_provider_arc(env).await?;
    Ok(Contract::new(token_address, erc20_abi()?, provider))
}

fn build_amount_quote(
    raw_market_amount: Option<&str>,
    market_currency_per_payment_token: Decimal,
    payment_token_decimals: u8,
    field_name: &str,
) -> Result<Option<MarketAmountQuote>, AuthError> {
    let Some(raw_market_amount) = raw_market_amount.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    let market_amount = Decimal::from_str(raw_market_amount)
        .map_err(|_| AuthError::bad_request(format!("invalid {field_name} decimal amount")))?;
    if market_amount < Decimal::ZERO {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be greater than or equal to zero",
        )));
    }

    let payment_token_amount = round_decimal(
        market_amount / market_currency_per_payment_token,
        payment_token_decimals.into(),
    );
    let payment_token_base_units =
        decimal_to_base_units(payment_token_amount, payment_token_decimals)?;

    Ok(Some(MarketAmountQuote {
        market_currency_amount: format_decimal(market_amount),
        payment_token_amount: format_decimal(payment_token_amount),
        payment_token_base_units,
    }))
}

fn read_decimal_field(
    entry: &serde_json::Map<String, Value>,
    field_name: &str,
) -> Result<Decimal, AuthError> {
    let value = entry.get(field_name).ok_or_else(|| {
        AuthError::bad_request(format!(
            "unsupported market_currency `{field_name}` for current CoinGecko response",
        ))
    })?;

    let number = match value {
        Value::Number(number) => number.to_string(),
        _ => {
            return Err(AuthError::internal(
                "CoinGecko field had invalid numeric shape",
                field_name.to_owned(),
            ));
        }
    };

    Decimal::from_str(&number)
        .map_err(|error| AuthError::internal("invalid CoinGecko decimal value", error))
}

fn normalize_market_currency(raw: &str) -> Result<String, AuthError> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(AuthError::bad_request("market_currency is required"));
    }
    if !normalized
        .chars()
        .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    {
        return Err(AuthError::bad_request(
            "market_currency must be lowercase letters/numbers only",
        ));
    }

    Ok(normalized)
}

fn decimal_to_base_units(value: Decimal, decimals: u8) -> Result<String, AuthError> {
    if decimals > 28 {
        return Err(AuthError::internal(
            "unsupported payment token decimals",
            format!("decimals {decimals} exceeds supported precision"),
        ));
    }

    let multiplier = decimal_power_of_ten(decimals);
    let scaled = round_decimal(value * multiplier, 0);
    let normalized = scaled.normalize().to_string();

    if normalized.contains('.') {
        return Err(AuthError::internal(
            "failed to derive integer payment token base units",
            format!("non-integer base unit value {normalized}"),
        ));
    }

    Ok(normalized)
}

fn decimal_power_of_ten(decimals: u8) -> Decimal {
    let mut value = Decimal::ONE;
    for _ in 0..decimals {
        value *= Decimal::TEN;
    }
    value
}

fn round_decimal(value: Decimal, scale: u32) -> Decimal {
    value.round_dp_with_strategy(scale, RoundingStrategy::MidpointAwayFromZero)
}

fn format_decimal(value: Decimal) -> String {
    value.normalize().to_string()
}
