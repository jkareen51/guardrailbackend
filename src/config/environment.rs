use std::{
    env,
    fmt::Display,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};
use dotenvy::dotenv;
use ethers_core::types::Address;

#[derive(Clone)]
pub struct Environment {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub db_max_connections: u32,
    pub db_acquire_timeout_ms: u64,
    pub cors_allowed_origins: Vec<String>,
    pub google_client_id: String,
    pub google_jwks_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_hours: i64,
    pub admin_wallet_addresses: Vec<String>,
    pub operator_private_key: Option<String>,
    pub admin_multisig_address: Option<String>,
    pub monad_rpc_url: String,
    pub monad_rpc_urls: Vec<String>,
    pub monad_chain_id: i64,
    pub access_control_address: String,
    pub asset_factory_address: String,
    pub compliance_registry_address: String,
    pub treasury_address: String,
    pub oracle_data_bridge_address: String,
    pub payment_token_address: String,
    pub exchange_rate_api_base_url: String,
    pub coinpaprika_api_base_url: String,
    pub coinpaprika_payment_token_coin_id: String,
    pub aa_bundler_rpc_url: String,
    pub aa_entry_point_address: String,
    pub aa_simple_account_factory_address: String,
    pub aa_user_operation_poll_interval_ms: u64,
    pub aa_user_operation_timeout_ms: u64,
    pub aa_owner_encryption_key: String,
    pub aa_owner_encryption_key_version: i32,
    pub payment_token_decimals: u8,
    pub faucet_usdc_cooldown_secs: i64,
    pub open_purchase_auto_whitelist: bool,
    pub filebase_bucket_name: Option<String>,
    pub filebase_s3_endpoint: Option<String>,
    pub filebase_region: Option<String>,
    pub filebase_access_key: Option<String>,
    pub filebase_secret_key: Option<String>,
    pub filebase_gateway_base_url: Option<String>,
    pub filebase_ipfs_rpc_url: Option<String>,
    pub filebase_ipfs_rpc_token: Option<String>,
}

impl Environment {
    pub fn load() -> Result<Self> {
        dotenv().ok();

        let monad_rpc_urls = parse_rpc_urls_env()?;
        let monad_rpc_url = monad_rpc_urls.first().cloned().ok_or_else(|| {
            anyhow!("missing required env var `MONAD_RPC_URL` or `MONAD_RPC_URLS`")
        })?;

        Ok(Self {
            host: parse_env("HOST", IpAddr::V4(Ipv4Addr::UNSPECIFIED))?,
            port: parse_env("PORT", 8080)?,
            database_url: required_env("DATABASE_URL")?,
            db_max_connections: parse_env("DB_MAX_CONNECTIONS", 20)?,
            db_acquire_timeout_ms: parse_env("DB_ACQUIRE_TIMEOUT_MS", 10_000)?,
            cors_allowed_origins: parse_cors_allowed_origins()?,
            google_client_id: required_env("GOOGLE_CLIENT_ID")?,
            google_jwks_url: parse_env(
                "GOOGLE_JWKS_URL",
                "https://www.googleapis.com/oauth2/v3/certs".to_owned(),
            )?,
            jwt_secret: required_env("JWT_SECRET")?,
            jwt_ttl_hours: parse_env("JWT_TTL_HOURS", 24)?,
            admin_wallet_addresses: parse_wallet_list_env("ADMIN_WALLET_ADDRESSES")?,
            operator_private_key: optional_env("OPERATOR_PRIVATE_KEY")
                .or_else(|| optional_env("MONAD_OPERATOR_PRIVATE_KEY")),
            admin_multisig_address: optional_any_address_env(&[
                "ADMIN_MULTISIG_ADDRESS",
                "MULTISIG_ADMIN_ADDRESS",
            ])?,
            monad_rpc_url,
            monad_rpc_urls,
            monad_chain_id: parse_env("MONAD_CHAIN_ID", 10143)?,
            access_control_address: required_address_env("ACCESS_CONTROL_ADDRESS")?,
            asset_factory_address: required_address_env("ASSET_FACTORY_ADDRESS")?,
            compliance_registry_address: required_any_address_env(&[
                "COMPLIANCE_REGISTRY_ADDRESS",
                "COMPLIANCE_DIAMOND_ADDRESS",
            ])?,
            treasury_address: required_address_env("TREASURY_ADDRESS")?,
            oracle_data_bridge_address: required_address_env("ORACLE_DATA_BRIDGE_ADDRESS")?,
            payment_token_address: required_any_address_env(&[
                "PAYMENT_TOKEN_ADDRESS",
                "MOCK_USDC_ADDRESS",
            ])?,
            exchange_rate_api_base_url: parse_env(
                "EXCHANGE_RATE_API_BASE_URL",
                "https://open.er-api.com/v6".to_owned(),
            )?,
            coinpaprika_api_base_url: parse_env(
                "COINPAPRIKA_API_BASE_URL",
                "https://api.coinpaprika.com/v1".to_owned(),
            )?,
            coinpaprika_payment_token_coin_id: parse_env(
                "COINPAPRIKA_PAYMENT_TOKEN_COIN_ID",
                "usdc-usd-coin".to_owned(),
            )?,
            aa_bundler_rpc_url: required_env("AA_BUNDLER_RPC_URL")?,
            aa_entry_point_address: required_address_env("AA_ENTRY_POINT_ADDRESS")?,
            aa_simple_account_factory_address: required_address_env(
                "AA_SIMPLE_ACCOUNT_FACTORY_ADDRESS",
            )?,
            aa_user_operation_poll_interval_ms: parse_env(
                "AA_USER_OPERATION_POLL_INTERVAL_MS",
                1_500,
            )?,
            aa_user_operation_timeout_ms: parse_env("AA_USER_OPERATION_TIMEOUT_MS", 120_000)?,
            aa_owner_encryption_key: required_env("AA_OWNER_ENCRYPTION_KEY")?,
            aa_owner_encryption_key_version: parse_env("AA_OWNER_ENCRYPTION_KEY_VERSION", 1)?,
            payment_token_decimals: parse_env("PAYMENT_TOKEN_DECIMALS", 6)?,
            faucet_usdc_cooldown_secs: parse_env("FAUCET_USDC_COOLDOWN_SECS", 3600)?,
            open_purchase_auto_whitelist: parse_env("OPEN_PURCHASE_AUTO_WHITELIST", false)?,
            filebase_bucket_name: optional_env("FILEBASE_BUCKET_NAME"),
            filebase_s3_endpoint: optional_env("FILEBASE_S3_ENDPOINT"),
            filebase_region: optional_env("FILEBASE_REGION"),
            filebase_access_key: optional_env("FILEBASE_ACCESS_KEY"),
            filebase_secret_key: optional_env("FILEBASE_SECRET_KEY"),
            filebase_gateway_base_url: optional_env("FILEBASE_GATEWAY_BASE_URL"),
            filebase_ipfs_rpc_url: optional_env("FILEBASE_IPFS_RPC_URL"),
            filebase_ipfs_rpc_token: optional_env("FILEBASE_IPFS_RPC_TOKEN"),
        })
    }

    pub fn bind_address(&self) -> SocketAddr {
        SocketAddr::from((self.host, self.port))
    }

    pub fn is_admin_wallet(&self, wallet_address: &str) -> bool {
        self.admin_wallet_addresses
            .iter()
            .any(|value| value == wallet_address)
    }
}

fn required_env(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("missing required env var `{key}`"))
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_cors_allowed_origins() -> Result<Vec<String>> {
    if let Some(origins) = optional_env("CORS_ALLOWED_ORIGINS") {
        return Ok(split_csv_values(&origins));
    }

    if let Some(origin) = optional_env("CORS_ALLOWED_ORIGIN") {
        return Ok(vec![origin]);
    }

    Ok(vec![
        "http://localhost:5173".to_owned(),
        "http://localhost:3000".to_owned(),
    ])
}

fn parse_rpc_urls_env() -> Result<Vec<String>> {
    let mut urls = if let Some(raw) = optional_env("MONAD_RPC_URLS") {
        split_csv_values(&raw)
    } else if let Some(raw) = optional_env("MONAD_RPC_URL") {
        vec![raw]
    } else {
        Vec::new()
    };

    if urls.is_empty() {
        return Err(anyhow!(
            "missing required env var `MONAD_RPC_URL` or `MONAD_RPC_URLS`"
        ));
    }

    for url in &urls {
        let trimmed = url.trim();
        if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
            return Err(anyhow!(
                "invalid MONAD_RPC_URL/MONAD_RPC_URLS entry `{trimmed}`: expected http(s) url"
            ));
        }
    }

    urls.dedup();
    Ok(urls)
}

fn parse_env<T>(key: &str, default: T) -> Result<T>
where
    T: FromStr + ToString,
    T::Err: Display,
{
    let raw = env::var(key).unwrap_or_else(|_| default.to_string());

    raw.parse::<T>()
        .map_err(|error| anyhow!("invalid value for env var `{key}`: {error}"))
}

fn split_csv_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_wallet_list_env(key: &str) -> Result<Vec<String>> {
    let raw = env::var(key).unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    split_csv_values(&raw)
        .into_iter()
        .map(|value| {
            normalize_address(&value)
                .map_err(|error| anyhow!("invalid wallet address in env var `{key}`: {error}"))
        })
        .collect()
}

fn required_address_env(key: &str) -> Result<String> {
    let raw = required_env(key)?;

    normalize_address(&raw).map_err(|error| anyhow!("invalid value for env var `{key}`: {error}"))
}

fn required_any_address_env(keys: &[&str]) -> Result<String> {
    for key in keys {
        if let Some(raw) = optional_env(key) {
            return normalize_address(&raw)
                .map_err(|error| anyhow!("invalid value for env var `{key}`: {error}"));
        }
    }

    Err(anyhow!(
        "missing required env var, expected one of: {}",
        keys.join(", ")
    ))
}

fn optional_any_address_env(keys: &[&str]) -> Result<Option<String>> {
    for key in keys {
        if let Some(raw) = optional_env(key) {
            return normalize_address(&raw)
                .map(Some)
                .map_err(|error| anyhow!("invalid value for env var `{key}`: {error}"));
        }
    }

    Ok(None)
}

fn normalize_address(raw: &str) -> Result<String> {
    let address: Address = raw
        .parse()
        .map_err(|error| anyhow!("invalid address `{raw}`: {error}"))?;

    Ok(format!("{address:#x}"))
}
