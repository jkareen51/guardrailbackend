use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        asset::model::{
            AssetCatalogRecord, AssetPriceHistoryRecord, AssetRecord, AssetTagCountRecord,
            AssetTypeRecord, PendingRedemptionRecord, UserTradeHistoryRecord,
        },
        auth::error::AuthError,
    },
};

use chrono::{DateTime, Utc};
use sqlx::{Postgres, QueryBuilder};

mod sql {
    pub const GET_ASSET_TYPE: &str = include_str!("sql/get_asset_type.sql");
    pub const LIST_ASSET_TYPES: &str = include_str!("sql/list_asset_types.sql");
    pub const UPSERT_ASSET_TYPE: &str = include_str!("sql/upsert_asset_type.sql");
    pub const GET_ASSET: &str = include_str!("sql/get_asset.sql");
    pub const GET_ASSET_BY_PROPOSAL: &str = include_str!("sql/get_asset_by_proposal.sql");
    pub const GET_ASSET_BY_SLUG: &str = include_str!("sql/get_asset_by_slug.sql");
    pub const UPSERT_ASSET: &str = include_str!("sql/upsert_asset.sql");
    pub const UPSERT_ASSET_CATALOG_ENTRY: &str = include_str!("sql/upsert_asset_catalog_entry.sql");
    pub const INSERT_ASSET_PRICE_HISTORY: &str = include_str!("sql/insert_asset_price_history.sql");
    pub const LIST_ASSET_PRICE_HISTORY: &str = include_str!("sql/list_asset_price_history.sql");
    pub const INSERT_TRADE_HISTORY: &str = include_str!("sql/insert_trade_history.sql");
    pub const LIST_USER_TRADE_HISTORY: &str = include_str!("sql/list_user_trade_history.sql");
    pub const LIST_PENDING_REDEMPTIONS_FOR_ASSET: &str =
        include_str!("sql/list_pending_redemptions_for_asset.sql");
}

pub struct AssetListFilters<'a> {
    pub chain_id: i64,
    pub asset_type_id: Option<&'a str>,
    pub tag_slug: Option<&'a str>,
    pub q: Option<&'a str>,
    pub asset_state: Option<i32>,
    pub self_service_purchase_enabled: Option<bool>,
    pub featured: Option<bool>,
    pub limit: i64,
    pub offset: i64,
    pub only_visible: bool,
    pub require_searchable: bool,
}

pub async fn get_asset_type(
    pool: &DbPool,
    asset_type_id: &str,
) -> Result<Option<AssetTypeRecord>, AuthError> {
    sqlx::query_as::<_, AssetTypeRecord>(sql::GET_ASSET_TYPE)
        .bind(asset_type_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_asset_types(pool: &DbPool) -> Result<Vec<AssetTypeRecord>, AuthError> {
    sqlx::query_as::<_, AssetTypeRecord>(sql::LIST_ASSET_TYPES)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_asset_type(
    pool: &DbPool,
    asset_type_id: &str,
    asset_type_name: &str,
    implementation_address: &str,
    is_registered: bool,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<AssetTypeRecord, AuthError> {
    sqlx::query_as::<_, AssetTypeRecord>(sql::UPSERT_ASSET_TYPE)
        .bind(asset_type_id)
        .bind(asset_type_name)
        .bind(implementation_address)
        .bind(is_registered)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_asset(
    pool: &DbPool,
    chain_id: i64,
    asset_address: &str,
) -> Result<Option<AssetRecord>, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::GET_ASSET)
        .bind(chain_id)
        .bind(asset_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_asset_by_slug(
    pool: &DbPool,
    chain_id: i64,
    slug: &str,
) -> Result<Option<AssetRecord>, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::GET_ASSET_BY_SLUG)
        .bind(chain_id)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_asset_by_proposal(
    pool: &DbPool,
    chain_id: i64,
    proposal_id: &str,
) -> Result<Option<AssetRecord>, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::GET_ASSET_BY_PROPOSAL)
        .bind(chain_id)
        .bind(proposal_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_assets(
    pool: &DbPool,
    filters: AssetListFilters<'_>,
) -> Result<Vec<AssetRecord>, AuthError> {
    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            a.asset_address,
            a.proposal_id,
            a.asset_type_id,
            t.asset_type_name,
            a.name,
            a.symbol,
            a.max_supply,
            a.total_supply,
            a.asset_state,
            a.asset_state_label,
            a.controllable,
            a.self_service_purchase_enabled,
            a.price_per_token,
            a.redemption_price_per_token,
            a.treasury_address,
            a.compliance_registry_address,
            a.payment_token_address,
            a.metadata_hash,
            c.slug,
            c.image_url,
            c.summary,
            c.market_segment,
            COALESCE(c.suggested_internal_tags, ARRAY[]::TEXT[]) AS suggested_internal_tags,
            COALESCE(c.sources, ARRAY[]::TEXT[]) AS sources,
            COALESCE(c.featured, FALSE) AS featured,
            COALESCE(c.visible, TRUE) AS visible,
            COALESCE(c.searchable, TRUE) AS searchable,
            a.holder_count,
            a.total_pending_redemptions,
            a.created_by_user_id,
            a.updated_by_user_id,
            a.last_tx_hash,
            a.created_at,
            a.updated_at
        FROM assets a
        LEFT JOIN asset_types t ON t.asset_type_id = a.asset_type_id
        LEFT JOIN asset_catalog_entries c ON c.asset_address = a.asset_address
        WHERE 1 = 1
        "#,
    );

    query.push(" AND a.chain_id = ");
    query.push_bind(filters.chain_id);

    if filters.only_visible {
        query.push(" AND COALESCE(c.visible, TRUE) = TRUE");
    }
    if filters.require_searchable {
        query.push(" AND COALESCE(c.searchable, TRUE) = TRUE");
    }
    if let Some(asset_type_id) = filters.asset_type_id {
        query.push(" AND a.asset_type_id = ");
        query.push_bind(asset_type_id);
    }
    if let Some(tag_slug) = filters.tag_slug {
        query.push(" AND COALESCE(c.suggested_internal_tags, ARRAY[]::TEXT[]) @> ARRAY[");
        query.push_bind(tag_slug);
        query.push("]::TEXT[]");
    }
    if let Some(asset_state) = filters.asset_state {
        query.push(" AND a.asset_state = ");
        query.push_bind(asset_state);
    }
    if let Some(enabled) = filters.self_service_purchase_enabled {
        query.push(" AND a.self_service_purchase_enabled = ");
        query.push_bind(enabled);
    }
    if let Some(featured) = filters.featured {
        query.push(" AND COALESCE(c.featured, FALSE) = ");
        query.push_bind(featured);
    }
    if let Some(q) = filters.q {
        let pattern = format!("%{}%", q.trim());
        query.push(" AND (a.name ILIKE ");
        query.push_bind(pattern.clone());
        query.push(" OR a.symbol ILIKE ");
        query.push_bind(pattern.clone());
        query.push(" OR COALESCE(c.slug, '') ILIKE ");
        query.push_bind(pattern.clone());
        query.push(" OR COALESCE(c.summary, '') ILIKE ");
        query.push_bind(pattern.clone());
        query.push(" OR COALESCE(c.market_segment, '') ILIKE ");
        query.push_bind(pattern.clone());
        query.push(
            " OR array_to_string(COALESCE(c.suggested_internal_tags, ARRAY[]::TEXT[]), ' ') ILIKE ",
        );
        query.push_bind(pattern);
        query.push(")");
    }

    query.push(" ORDER BY COALESCE(c.featured, FALSE) DESC, a.updated_at DESC");
    query.push(" LIMIT ");
    query.push_bind(filters.limit);
    query.push(" OFFSET ");
    query.push_bind(filters.offset);

    query
        .build_query_as::<AssetRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_assets_by_type(
    pool: &DbPool,
    chain_id: i64,
    asset_type_id: &str,
) -> Result<Vec<AssetRecord>, AuthError> {
    list_assets(
        pool,
        AssetListFilters {
            chain_id,
            asset_type_id: Some(asset_type_id),
            tag_slug: None,
            q: None,
            asset_state: None,
            self_service_purchase_enabled: None,
            featured: None,
            limit: 100,
            offset: 0,
            only_visible: true,
            require_searchable: false,
        },
    )
    .await
}

pub async fn list_asset_tags(
    pool: &DbPool,
    chain_id: i64,
) -> Result<Vec<AssetTagCountRecord>, AuthError> {
    sqlx::query_as::<_, AssetTagCountRecord>(
        r#"
        SELECT
            tag.slug AS slug,
            COUNT(*)::BIGINT AS asset_count
        FROM assets a
        JOIN asset_catalog_entries c ON c.asset_address = a.asset_address
        CROSS JOIN LATERAL unnest(COALESCE(c.suggested_internal_tags, ARRAY[]::TEXT[])) AS tag(slug)
        WHERE a.chain_id = $1
          AND COALESCE(c.visible, TRUE) = TRUE
          AND COALESCE(c.searchable, TRUE) = TRUE
        GROUP BY tag.slug
        ORDER BY COUNT(*) DESC, tag.slug ASC
        "#,
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await
    .map_err(AuthError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_asset(
    pool: &DbPool,
    asset_address: &str,
    chain_id: i64,
    proposal_id: &str,
    asset_type_id: &str,
    name: &str,
    symbol: &str,
    max_supply: &str,
    total_supply: &str,
    asset_state: i32,
    asset_state_label: &str,
    controllable: bool,
    self_service_purchase_enabled: bool,
    price_per_token: &str,
    redemption_price_per_token: &str,
    treasury_address: &str,
    compliance_registry_address: &str,
    payment_token_address: &str,
    metadata_hash: &str,
    holder_count: &str,
    total_pending_redemptions: &str,
    created_by_user_id: Option<Uuid>,
    updated_by_user_id: Option<Uuid>,
    last_tx_hash: Option<&str>,
) -> Result<AssetRecord, AuthError> {
    sqlx::query_as::<_, AssetRecord>(sql::UPSERT_ASSET)
        .bind(asset_address)
        .bind(chain_id)
        .bind(proposal_id)
        .bind(asset_type_id)
        .bind(name)
        .bind(symbol)
        .bind(max_supply)
        .bind(total_supply)
        .bind(asset_state)
        .bind(asset_state_label)
        .bind(controllable)
        .bind(self_service_purchase_enabled)
        .bind(price_per_token)
        .bind(redemption_price_per_token)
        .bind(treasury_address)
        .bind(compliance_registry_address)
        .bind(payment_token_address)
        .bind(metadata_hash)
        .bind(holder_count)
        .bind(total_pending_redemptions)
        .bind(created_by_user_id)
        .bind(updated_by_user_id)
        .bind(last_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_asset_catalog_entry(
    pool: &DbPool,
    asset_address: &str,
    slug: &str,
    image_url: Option<&str>,
    summary: Option<&str>,
    market_segment: Option<&str>,
    suggested_internal_tags: &[String],
    sources: &[String],
    featured: bool,
    visible: bool,
    searchable: bool,
    created_by_user_id: Option<Uuid>,
    updated_by_user_id: Option<Uuid>,
) -> Result<AssetCatalogRecord, AuthError> {
    sqlx::query_as::<_, AssetCatalogRecord>(sql::UPSERT_ASSET_CATALOG_ENTRY)
        .bind(asset_address)
        .bind(slug)
        .bind(image_url)
        .bind(summary)
        .bind(market_segment)
        .bind(suggested_internal_tags)
        .bind(sources)
        .bind(featured)
        .bind(visible)
        .bind(searchable)
        .bind(created_by_user_id)
        .bind(updated_by_user_id)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn insert_asset_price_history(
    pool: &DbPool,
    asset_address: &str,
    price_per_token: &str,
    redemption_price_per_token: &str,
    source: &str,
    tx_hash: Option<&str>,
    created_by_user_id: Option<Uuid>,
    observed_at: Option<DateTime<Utc>>,
) -> Result<(), AuthError> {
    sqlx::query(sql::INSERT_ASSET_PRICE_HISTORY)
        .bind(asset_address)
        .bind(price_per_token)
        .bind(redemption_price_per_token)
        .bind(source)
        .bind(tx_hash)
        .bind(created_by_user_id)
        .bind(observed_at)
        .execute(pool)
        .await
        .map_err(AuthError::from)?;

    Ok(())
}

pub async fn list_asset_price_history(
    pool: &DbPool,
    asset_address: &str,
    observed_from: Option<DateTime<Utc>>,
) -> Result<Vec<AssetPriceHistoryRecord>, AuthError> {
    sqlx::query_as::<_, AssetPriceHistoryRecord>(sql::LIST_ASSET_PRICE_HISTORY)
        .bind(asset_address)
        .bind(observed_from)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn insert_trade_history(
    pool: &DbPool,
    user_id: Uuid,
    wallet_address: &str,
    asset_address: &str,
    trade_type: &str,
    token_amount: &str,
    payment_amount: &str,
    price_per_token: &str,
    tx_hash: &str,
) -> Result<Option<UserTradeHistoryRecord>, AuthError> {
    sqlx::query_as::<_, UserTradeHistoryRecord>(sql::INSERT_TRADE_HISTORY)
        .bind(user_id)
        .bind(wallet_address)
        .bind(asset_address)
        .bind(trade_type)
        .bind(token_amount)
        .bind(payment_amount)
        .bind(price_per_token)
        .bind(tx_hash)
        .fetch_optional(pool)
        .await
        .map_err(|error| AuthError::internal("failed to insert trade history", error))
}

pub async fn list_user_trade_history(
    pool: &DbPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserTradeHistoryRecord>, AuthError> {
    sqlx::query_as::<_, UserTradeHistoryRecord>(sql::LIST_USER_TRADE_HISTORY)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|error| AuthError::internal("failed to list user trade history", error))
}

pub async fn list_pending_redemptions_for_asset(
    pool: &DbPool,
    asset_address: &str,
) -> Result<Vec<PendingRedemptionRecord>, AuthError> {
    sqlx::query_as::<_, PendingRedemptionRecord>(sql::LIST_PENDING_REDEMPTIONS_FOR_ASSET)
        .bind(asset_address)
        .fetch_all(pool)
        .await
        .map_err(|error| AuthError::internal("failed to list pending redemptions for asset", error))
}
