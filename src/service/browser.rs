use tokio::task::JoinSet;

use anyhow::Result;
use ethers_contract::Contract;
use ethers_core::types::{Address, U256};
use ethers_providers::{Http, Provider};

use crate::{
    app::AppState,
    module::{
        asset::{crud as asset_crud, model::AssetRecord},
        auth::{crud as auth_crud, error::AuthError},
        market::schema::{
            CategoriesResponse, CategorySummaryResponse, EventDetailResponse,
            EventListResponse, EventMarketsResponse, EventOnChainResponse, EventResponse,
            ListEventsQuery, MarketListResponse, MarketResponse, MarketsHomeQuery,
            MarketsHomeResponse, MyPortfolioResponse, PortfolioMarketSummaryResponse,
            PortfolioSummaryResponse, PositionOutcomeResponse, PublicEventCardResponse,
            PublicEventTeaserResponse, PublicMarketCardResponse, SearchMarketsQuery,
            TagSummaryResponse, TagsResponse,
        },
    },
    service::{
        asset::abi::{base_asset_token_abi, erc20_abi},
        chain::parse_address,
        rpc,
    },
};

const DEFAULT_SEARCH_LIMIT: i64 = 8;
const MAX_SEARCH_LIMIT: i64 = 50;
const DEFAULT_PORTFOLIO_ASSET_BATCH: i64 = 250;
const ASSET_TOKEN_DECIMALS: usize = 18;
const PAYMENT_TOKEN_FALLBACK_DECIMALS: usize = 6;

struct NormalizedSearchQuery {
    q: Option<String>,
    category_slug: Option<String>,
    subcategory_slug: Option<String>,
    tag_slug: Option<String>,
    trading_status: Option<String>,
    limit: i64,
    offset: i64,
}

struct PortfolioPositionRow {
    portfolio_value_base_units: U256,
    summary: PortfolioMarketSummaryResponse,
}

pub async fn search_markets(
    state: &AppState,
    query: SearchMarketsQuery,
) -> Result<MarketListResponse, AuthError> {
    let normalized = normalize_search_query(query)?;
    let scan_limit = ((normalized.limit + normalized.offset).max(normalized.limit) * 4)
        .clamp(normalized.limit, 250);

    let assets = asset_crud::list_assets(
        &state.db,
        asset_crud::AssetListFilters {
            chain_id: state.env.monad_chain_id,
            asset_type_id: None,
            tag_slug: normalized.tag_slug.as_deref(),
            q: normalized.q.as_deref(),
            asset_state: None,
            self_service_purchase_enabled: None,
            featured: None,
            limit: scan_limit,
            offset: 0,
            only_visible: true,
            require_searchable: true,
        },
    )
    .await?;

    let markets = assets
        .into_iter()
        .filter(|asset| asset_matches_browser_filters(asset, &normalized))
        .skip(normalized.offset as usize)
        .take(normalized.limit as usize)
        .map(asset_to_public_market_card)
        .collect::<Vec<_>>();

    Ok(MarketListResponse {
        markets,
        limit: normalized.limit,
        offset: normalized.offset,
    })
}

pub async fn list_tags(state: &AppState) -> Result<TagsResponse, AuthError> {
    let tags = asset_crud::list_asset_tags(&state.db, state.env.monad_chain_id)
        .await?
        .into_iter()
        .map(|record| TagSummaryResponse {
            label: label_from_slug(&record.slug),
            slug: record.slug,
            event_count: record.asset_count,
            market_count: record.asset_count,
        })
        .collect();

    Ok(TagsResponse { tags })
}

pub async fn get_my_portfolio(
    state: &AppState,
    user_id: uuid::Uuid,
) -> Result<MyPortfolioResponse, AuthError> {
    let wallet = auth_crud::get_wallet_for_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::forbidden("authenticated user does not have a linked wallet"))?;
    let wallet_address = parse_address(&wallet.wallet_address)?;
    let payment_token_address = parse_address(&state.env.payment_token_address)?;
    let payment_token_decimals = read_payment_token_decimals(state, payment_token_address)
        .await
        .unwrap_or(PAYMENT_TOKEN_FALLBACK_DECIMALS);
    let cash_balance_base_units =
        read_erc20_balance(state, payment_token_address, wallet_address).await?;
    let assets = load_all_assets_for_portfolio(state).await?;

    let mut positions = JoinSet::new();
    for asset in assets {
        let env = state.env.clone();
        positions.spawn(async move {
            build_portfolio_position(env, asset, wallet_address, payment_token_decimals).await
        });
    }

    let mut markets = Vec::new();
    let mut portfolio_balance_base_units = U256::zero();

    while let Some(result) = positions.join_next().await {
        match result {
            Ok(Ok(Some(position))) => {
                portfolio_balance_base_units += position.portfolio_value_base_units;
                markets.push(position);
            }
            Ok(Ok(None)) => {}
            Ok(Err(error)) => {
                tracing::warn!(%error, wallet_address = %wallet.wallet_address, "skipping portfolio asset row after read failure");
            }
            Err(error) => {
                tracing::warn!(%error, wallet_address = %wallet.wallet_address, "skipping portfolio asset row after task failure");
            }
        }
    }

    markets.sort_by(|left, right| {
        right
            .portfolio_value_base_units
            .cmp(&left.portfolio_value_base_units)
    });

    let market_rows = markets
        .into_iter()
        .map(|row| row.summary)
        .collect::<Vec<_>>();
    let total_balance_base_units = cash_balance_base_units + portfolio_balance_base_units;

    Ok(MyPortfolioResponse {
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        summary: PortfolioSummaryResponse {
            cash_balance: format_display_units(cash_balance_base_units, payment_token_decimals),
            portfolio_balance: format_display_units(
                portfolio_balance_base_units,
                payment_token_decimals,
            ),
            total_balance: format_display_units(total_balance_base_units, payment_token_decimals),
            total_buy_amount: "0".to_owned(),
            total_sell_amount: "0".to_owned(),
        },
        markets: market_rows,
        history: Vec::new(),
    })
}

pub async fn list_categories(state: &AppState) -> Result<CategoriesResponse, AuthError> {
    let assets = load_visible_searchable_assets(state).await?;
    let mut category_map: std::collections::HashMap<String, CategorySummaryResponse> =
        std::collections::HashMap::new();

    for asset in &assets {
        let slug = primary_category_slug(asset);
        let label = label_from_slug(&slug);

        let entry = category_map.entry(slug.clone()).or_insert_with(|| {
            CategorySummaryResponse {
                slug: slug.clone(),
                label,
                event_count: 0,
                market_count: 0,
                featured_event_count: 0,
                breaking_event_count: 0,
            }
        });

        entry.event_count += 1;
        entry.market_count += 1;
        if asset.featured {
            entry.featured_event_count += 1;
        }
    }

    let mut categories: Vec<CategorySummaryResponse> = category_map.into_values().collect();
    categories.sort_by(|left, right| {
        right
            .featured_event_count
            .cmp(&left.featured_event_count)
            .then(right.event_count.cmp(&left.event_count))
            .then(left.label.cmp(&right.label))
    });

    Ok(CategoriesResponse { categories })
}

pub async fn list_events(
    state: &AppState,
    query: ListEventsQuery,
) -> Result<EventListResponse, AuthError> {
    let limit = normalize_limit(query.limit)?;
    let offset = normalize_offset(query.offset)?;
    let include_markets = query.include_markets.unwrap_or(false);
    let tag_slug = normalize_optional_text(query.tag_slug);
    let category_slug = normalize_optional_text(query.category_slug)
        .map(|v| slugify_text(&v));
    let subcategory_slug = normalize_optional_text(query.subcategory_slug)
        .map(|v| slugify_text(&v));
    let featured_filter = query.featured;
    let breaking_filter = query.breaking;

    let scan_limit = ((limit + offset).max(limit) * 4).clamp(limit, 250);

    let assets = asset_crud::list_assets(
        &state.db,
        asset_crud::AssetListFilters {
            chain_id: state.env.monad_chain_id,
            asset_type_id: None,
            tag_slug: tag_slug.as_deref(),
            q: None,
            asset_state: None,
            self_service_purchase_enabled: None,
            featured: if featured_filter.unwrap_or(false) || breaking_filter.unwrap_or(false) {
                Some(true)
            } else {
                None
            },
            limit: scan_limit,
            offset: 0,
            only_visible: true,
            require_searchable: true,
        },
    )
    .await?;

    let events = assets
        .into_iter()
        .filter(|asset| {
            if let Some(ref cat_slug) = category_slug {
                if primary_category_slug(asset) != *cat_slug {
                    return false;
                }
            }
            if let Some(ref sub_slug) = subcategory_slug {
                let in_tags = asset
                    .suggested_internal_tags
                    .iter()
                    .map(|v| slugify_text(v))
                    .any(|v| v == *sub_slug);
                if !in_tags {
                    return false;
                }
            }
            if let Some(ref tag) = tag_slug {
                let normalized_tag = slugify_text(tag);
                let in_tags = asset
                    .suggested_internal_tags
                    .iter()
                    .map(|v| slugify_text(v))
                    .any(|v| v == normalized_tag);
                if !in_tags {
                    return false;
                }
            }
            true
        })
        .skip(offset as usize)
        .take(limit as usize)
        .map(|asset| {
            let markets = if include_markets {
                Some(vec![asset_to_public_market_card(asset.clone())])
            } else {
                None
            };

            PublicEventCardResponse {
                id: asset.asset_address.clone(),
                title: asset.name.clone(),
                slug: asset_slug(&asset),
                category_slug: primary_category_slug(&asset),
                subcategory_slug: None,
                tag_slugs: asset
                    .suggested_internal_tags
                    .iter()
                    .map(|v| slugify_text(v))
                    .collect(),
                image_url: asset.image_url.clone(),
                summary: asset.summary.clone(),
                featured: asset.featured,
                breaking: false,
                neg_risk: false,
                starts_at: None,
                sort_at: Some(asset.updated_at),
                market_count: 1,
                markets,
            }
        })
        .collect::<Vec<_>>();

    Ok(EventListResponse {
        events,
        limit,
        offset,
    })
}

pub async fn fetch_markets_home(
    state: &AppState,
    query: MarketsHomeQuery,
) -> Result<MarketsHomeResponse, AuthError> {
    let limit = query.limit.unwrap_or(24).clamp(1, 50);

    let featured = asset_crud::list_assets(
        &state.db,
        asset_crud::AssetListFilters {
            chain_id: state.env.monad_chain_id,
            asset_type_id: None,
            tag_slug: None,
            q: None,
            asset_state: None,
            self_service_purchase_enabled: None,
            featured: Some(true),
            limit,
            offset: 0,
            only_visible: true,
            require_searchable: true,
        },
    )
    .await?
    .into_iter()
    .map(asset_to_public_market_card)
    .collect::<Vec<_>>();

    let newest = asset_crud::list_assets(
        &state.db,
        asset_crud::AssetListFilters {
            chain_id: state.env.monad_chain_id,
            asset_type_id: None,
            tag_slug: None,
            q: None,
            asset_state: None,
            self_service_purchase_enabled: None,
            featured: None,
            limit,
            offset: 0,
            only_visible: true,
            require_searchable: true,
        },
    )
    .await?
    .into_iter()
    .map(asset_to_public_market_card)
    .collect::<Vec<_>>();

    Ok(MarketsHomeResponse {
        featured,
        breaking: Vec::new(),
        newest,
    })
}

pub async fn fetch_event(
    state: &AppState,
    event_id: &str,
) -> Result<EventDetailResponse, AuthError> {
    let asset = asset_crud::get_asset(&state.db, state.env.monad_chain_id, event_id)
        .await?
        .ok_or_else(|| AuthError::not_found(format!("event not found: {event_id}")))?;

    Ok(EventDetailResponse {
        event: asset_to_event_response(&asset),
        on_chain: asset_to_event_on_chain_response(&asset),
        markets_count: 1,
    })
}

pub async fn fetch_event_markets(
    state: &AppState,
    event_id: &str,
) -> Result<EventMarketsResponse, AuthError> {
    let asset = asset_crud::get_asset(&state.db, state.env.monad_chain_id, event_id)
        .await?
        .ok_or_else(|| AuthError::not_found(format!("event not found: {event_id}")))?;

    Ok(EventMarketsResponse {
        event: asset_to_event_response(&asset),
        on_chain: asset_to_event_on_chain_response(&asset),
        markets: vec![asset_to_market_response(&asset)],
    })
}

async fn load_visible_searchable_assets(state: &AppState) -> Result<Vec<AssetRecord>, AuthError> {
    let mut offset = 0_i64;
    let mut all_assets = Vec::new();
    let batch_size: i64 = 250;

    loop {
        let batch = asset_crud::list_assets(
            &state.db,
            asset_crud::AssetListFilters {
                chain_id: state.env.monad_chain_id,
                asset_type_id: None,
                tag_slug: None,
                q: None,
                asset_state: None,
                self_service_purchase_enabled: None,
                featured: None,
                limit: batch_size,
                offset,
                only_visible: true,
                require_searchable: true,
            },
        )
        .await?;

        let batch_len = batch.len() as i64;
        all_assets.extend(batch);

        if batch_len < batch_size {
            break;
        }

        offset += batch_size;
    }

    Ok(all_assets)
}

async fn load_all_assets_for_portfolio(state: &AppState) -> Result<Vec<AssetRecord>, AuthError> {
    let mut offset = 0_i64;
    let mut all_assets = Vec::new();

    loop {
        let batch = asset_crud::list_assets(
            &state.db,
            asset_crud::AssetListFilters {
                chain_id: state.env.monad_chain_id,
                asset_type_id: None,
                tag_slug: None,
                q: None,
                asset_state: None,
                self_service_purchase_enabled: None,
                featured: None,
                limit: DEFAULT_PORTFOLIO_ASSET_BATCH,
                offset,
                only_visible: false,
                require_searchable: false,
            },
        )
        .await?;

        let batch_len = batch.len() as i64;
        all_assets.extend(batch);

        if batch_len < DEFAULT_PORTFOLIO_ASSET_BATCH {
            break;
        }

        offset += DEFAULT_PORTFOLIO_ASSET_BATCH;
    }

    Ok(all_assets)
}

async fn build_portfolio_position(
    env: crate::config::environment::Environment,
    asset: AssetRecord,
    wallet_address: Address,
    payment_token_decimals: usize,
) -> Result<Option<PortfolioPositionRow>, AuthError> {
    let asset_address = parse_address(&asset.asset_address)?;
    let balance = read_asset_balance(&env, asset_address, wallet_address).await?;

    if balance.is_zero() {
        return Ok(None);
    }

    let price_per_token = U256::from_dec_str(&asset.price_per_token)
        .map_err(|error| AuthError::internal("invalid asset price_per_token", error))?;
    let portfolio_value_base_units = (balance * price_per_token) / U256::exp10(18);
    let event = asset_to_event_response(&asset);
    let on_chain = asset_to_event_on_chain_response(&asset);
    let market = asset_to_market_response(&asset);

    Ok(Some(PortfolioPositionRow {
        portfolio_value_base_units,
        summary: PortfolioMarketSummaryResponse {
            event,
            on_chain,
            market,
            buy_amount: "0".to_owned(),
            sell_amount: "0".to_owned(),
            portfolio_balance: format_display_units(
                portfolio_value_base_units,
                payment_token_decimals,
            ),
            positions: vec![PositionOutcomeResponse {
                outcome_index: 0,
                outcome_label: "Position".to_owned(),
                token_amount: format_display_units(balance, ASSET_TOKEN_DECIMALS),
                estimated_value_usdc: Some(format_display_units(
                    portfolio_value_base_units,
                    payment_token_decimals,
                )),
            }],
            last_traded_at: Some(asset.updated_at),
        },
    }))
}

async fn read_asset_balance(
    env: &crate::config::environment::Environment,
    asset_address: Address,
    wallet_address: Address,
) -> Result<U256, AuthError> {
    let provider = rpc::monad_provider_arc(env)
        .await
        .map_err(|error| AuthError::internal("failed to build Monad provider", error))?;
    let contract = Contract::<Provider<Http>>::new(
        asset_address,
        base_asset_token_abi()
            .map_err(|error| AuthError::internal("failed to build asset token ABI", error))?,
        provider,
    );

    contract
        .method::<_, U256>("balanceOf", wallet_address)
        .map_err(|error| AuthError::internal("failed to build asset balanceOf call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call asset balanceOf", error))
}

async fn read_erc20_balance(
    state: &AppState,
    token_address: Address,
    wallet_address: Address,
) -> Result<U256, AuthError> {
    let provider = rpc::monad_provider_arc(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build Monad provider", error))?;
    let contract = Contract::<Provider<Http>>::new(
        token_address,
        erc20_abi().map_err(|error| AuthError::internal("failed to build ERC20 ABI", error))?,
        provider,
    );

    contract
        .method::<_, U256>("balanceOf", wallet_address)
        .map_err(|error| {
            AuthError::internal("failed to build payment token balanceOf call", error)
        })?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call payment token balanceOf", error))
}

async fn read_payment_token_decimals(
    state: &AppState,
    token_address: Address,
) -> Result<usize, AuthError> {
    let provider = rpc::monad_provider_arc(&state.env)
        .await
        .map_err(|error| AuthError::internal("failed to build Monad provider", error))?;
    let contract = Contract::<Provider<Http>>::new(
        token_address,
        erc20_abi().map_err(|error| AuthError::internal("failed to build ERC20 ABI", error))?,
        provider,
    );
    let decimals = contract
        .method::<_, u8>("decimals", ())
        .map_err(|error| AuthError::internal("failed to build payment token decimals call", error))?
        .call()
        .await
        .map_err(|error| AuthError::internal("failed to call payment token decimals", error))?;

    Ok(usize::from(decimals))
}

fn asset_matches_browser_filters(asset: &AssetRecord, query: &NormalizedSearchQuery) -> bool {
    if let Some(trading_status) = &query.trading_status {
        if asset.asset_state_label.trim().to_ascii_lowercase() != *trading_status {
            return false;
        }
    }

    if let Some(category_slug) = &query.category_slug {
        let category = primary_category_slug(asset);
        if category != *category_slug {
            return false;
        }
    }

    if let Some(subcategory_slug) = &query.subcategory_slug {
        let matches_tag = asset
            .suggested_internal_tags
            .iter()
            .map(|value| slugify_text(value))
            .any(|value| value == *subcategory_slug);
        if !matches_tag {
            return false;
        }
    }

    true
}

fn normalize_search_query(query: SearchMarketsQuery) -> Result<NormalizedSearchQuery, AuthError> {
    Ok(NormalizedSearchQuery {
        q: normalize_optional_text(query.q),
        category_slug: normalize_optional_text(query.category_slug)
            .map(|value| slugify_text(&value)),
        subcategory_slug: normalize_optional_text(query.subcategory_slug)
            .map(|value| slugify_text(&value)),
        tag_slug: normalize_optional_text(query.tag_slug).map(|value| value.to_ascii_lowercase()),
        trading_status: normalize_optional_text(query.trading_status)
            .map(|value| value.to_ascii_lowercase()),
        limit: normalize_limit(query.limit)?,
        offset: normalize_offset(query.offset)?,
    })
}

fn normalize_optional_text(raw: Option<String>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn normalize_limit(raw: Option<i64>) -> Result<i64, AuthError> {
    let limit = raw.unwrap_or(DEFAULT_SEARCH_LIMIT);
    if !(1..=MAX_SEARCH_LIMIT).contains(&limit) {
        return Err(AuthError::bad_request(format!(
            "limit must be between 1 and {MAX_SEARCH_LIMIT}",
        )));
    }

    Ok(limit)
}

fn normalize_offset(raw: Option<i64>) -> Result<i64, AuthError> {
    let offset = raw.unwrap_or(0);
    if offset < 0 {
        return Err(AuthError::bad_request(
            "offset must be greater than or equal to zero",
        ));
    }

    Ok(offset)
}

fn asset_to_public_market_card(asset: AssetRecord) -> PublicMarketCardResponse {
    PublicMarketCardResponse {
        id: asset.asset_address.clone(),
        slug: asset_slug(&asset),
        label: asset.symbol.clone(),
        question: asset.name.clone(),
        question_id: asset.proposal_id.clone(),
        condition_id: Some(asset.asset_address.clone()),
        market_type: asset_market_type_label(&asset),
        outcomes: vec!["Buy".to_owned(), "Sell".to_owned()],
        end_time: asset.updated_at,
        sort_order: 0,
        trading_status: asset.asset_state_label.to_ascii_lowercase(),
        current_prices: None,
        stats: None,
        quote_summary: None,
        event: asset_to_public_event_teaser(&asset),
    }
}

fn asset_to_public_event_teaser(asset: &AssetRecord) -> PublicEventTeaserResponse {
    PublicEventTeaserResponse {
        id: asset.asset_address.clone(),
        title: asset.name.clone(),
        slug: asset_slug(asset),
        category_slug: primary_category_slug(asset),
        subcategory_slug: None,
        tag_slugs: asset
            .suggested_internal_tags
            .iter()
            .map(|value| slugify_text(value))
            .collect(),
        image_url: asset.image_url.clone(),
        summary: asset.summary.clone(),
        featured: asset.featured,
        breaking: false,
        neg_risk: false,
    }
}

fn asset_to_event_response(asset: &AssetRecord) -> EventResponse {
    EventResponse {
        title: asset.name.clone(),
        slug: asset_slug(asset),
        category_slug: primary_category_slug(asset),
        subcategory_slug: None,
        tag_slugs: asset
            .suggested_internal_tags
            .iter()
            .map(|value| slugify_text(value))
            .collect(),
        image_url: asset.image_url.clone(),
        summary: asset.summary.clone(),
        rules: String::new(),
        context: asset.market_segment.clone(),
        additional_context: None,
        resolution_sources: asset.sources.clone(),
        resolution_timezone: "UTC".to_owned(),
        starts_at: None,
        sort_at: Some(asset.updated_at),
        featured: asset.featured,
        breaking: false,
        searchable: asset.searchable,
        visible: asset.visible,
        hide_resolved_by_default: false,
        publication_status: "published".to_owned(),
    }
}

fn asset_to_event_on_chain_response(asset: &AssetRecord) -> EventOnChainResponse {
    EventOnChainResponse {
        event_id: asset.proposal_id.clone(),
        group_id: asset.asset_type_id.clone(),
        series_id: asset.asset_address.clone(),
        neg_risk: false,
        tx_hash: asset.last_tx_hash.clone(),
    }
}

fn asset_to_market_response(asset: &AssetRecord) -> MarketResponse {
    MarketResponse {
        id: asset.asset_address.clone(),
        slug: asset_slug(asset),
        label: asset.symbol.clone(),
        question: asset.name.clone(),
        question_id: asset.proposal_id.clone(),
        condition_id: Some(asset.asset_address.clone()),
        market_type: asset_market_type_label(asset),
        outcomes: vec!["Position".to_owned()],
        end_time: asset.updated_at,
        sort_order: 0,
        publication_status: "published".to_owned(),
        trading_status: asset.asset_state_label.to_ascii_lowercase(),
        current_prices: None,
        stats: None,
        quote_summary: None,
    }
}

fn asset_market_type_label(asset: &AssetRecord) -> String {
    asset.asset_type_name.clone().unwrap_or_else(|| {
        asset
            .market_segment
            .clone()
            .unwrap_or_else(|| asset.asset_type_id.clone())
    })
}

fn asset_slug(asset: &AssetRecord) -> String {
    asset
        .slug
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| asset.asset_address.clone())
}

fn primary_category_slug(asset: &AssetRecord) -> String {
    asset
        .market_segment
        .as_deref()
        .map(slugify_text)
        .filter(|value| !value.is_empty())
        .or_else(|| asset.asset_type_name.as_deref().map(slugify_text))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "assets".to_owned())
}

fn slugify_text(raw: &str) -> String {
    let mut slug = String::with_capacity(raw.len());
    let mut previous_was_hyphen = false;

    for character in raw.trim().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_was_hyphen = false;
            continue;
        }

        if !previous_was_hyphen {
            slug.push('-');
            previous_was_hyphen = true;
        }
    }

    slug.trim_matches('-').to_owned()
}

fn label_from_slug(raw: &str) -> String {
    raw.split(['-', '_', ' '])
        .filter(|token| !token.is_empty())
        .map(|token| {
            if token.len() <= 4 {
                token.to_ascii_uppercase()
            } else {
                let mut characters = token.chars();
                match characters.next() {
                    Some(first) => format!(
                        "{}{}",
                        first.to_ascii_uppercase(),
                        characters.as_str().to_ascii_lowercase()
                    ),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_display_units(value: U256, decimals: usize) -> String {
    if value.is_zero() {
        return "0".to_owned();
    }

    let digits = value.to_string();

    if decimals == 0 {
        return digits;
    }

    if digits.len() <= decimals {
        let fractional = format!("{:0>width$}", digits, width = decimals)
            .trim_end_matches('0')
            .to_owned();
        return if fractional.is_empty() {
            "0".to_owned()
        } else {
            format!("0.{fractional}")
        };
    }

    let split_index = digits.len() - decimals;
    let whole = &digits[..split_index];
    let fractional = digits[split_index..].trim_end_matches('0');

    if fractional.is_empty() {
        whole.to_owned()
    } else {
        format!("{whole}.{fractional}")
    }
}
