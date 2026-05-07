use chrono::{NaiveDate, Utc};
use reqwest::Url;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        asset::schema::AdminCreateAssetRequest,
        asset_request::{
            crud,
            model::{AssetRequestRecord, NewAssetRequestRecord},
            schema::{
                AdminUpdateAssetRequestStatusRequest, AssetRequestDeployResponse,
                AssetRequestListResponse, AssetRequestResponse, CreateAssetRequestRequest,
                ListAssetRequestsQuery,
            },
        },
        auth::{crud as auth_crud, error::AuthError},
    },
    service::{
        asset,
        chain::{parse_bytes32_input, parse_u256},
    },
};

const DEFAULT_LIST_LIMIT: i64 = 20;
const MAX_LIST_LIMIT: i64 = 100;
const MAX_TAGS: usize = 12;
const MAX_URLS: usize = 20;
const MAX_NOTES_LEN: usize = 4_000;

pub async fn create_asset_request(
    state: &AppState,
    user_id: Uuid,
    payload: CreateAssetRequestRequest,
) -> Result<AssetRequestResponse, AuthError> {
    let normalized = normalize_new_request(user_id, payload)?;
    let record = crud::create_asset_request(&state.db, &normalized).await?;
    Ok(AssetRequestResponse::from(record))
}

pub async fn list_my_asset_requests(
    state: &AppState,
    user_id: Uuid,
    query: ListAssetRequestsQuery,
) -> Result<AssetRequestListResponse, AuthError> {
    let (status, limit, offset) = normalize_list_query(query)?;
    let records = crud::list_asset_requests_for_submitter(
        &state.db,
        user_id,
        status.as_deref(),
        limit,
        offset,
    )
    .await?;

    Ok(AssetRequestListResponse {
        asset_requests: records
            .into_iter()
            .map(AssetRequestResponse::from)
            .collect(),
        limit,
        offset,
    })
}

pub async fn get_asset_request(
    state: &AppState,
    user_id: Uuid,
    request_id: &str,
) -> Result<AssetRequestResponse, AuthError> {
    let request_id = parse_request_id(request_id)?;
    let record = crud::get_asset_request(&state.db, request_id)
        .await?
        .ok_or_else(|| AuthError::not_found("asset request not found"))?;

    ensure_request_access(state, user_id, &record).await?;

    Ok(AssetRequestResponse::from(record))
}

pub async fn list_asset_requests(
    state: &AppState,
    query: ListAssetRequestsQuery,
) -> Result<AssetRequestListResponse, AuthError> {
    let (status, limit, offset) = normalize_list_query(query)?;
    let records = crud::list_asset_requests(&state.db, status.as_deref(), limit, offset).await?;

    Ok(AssetRequestListResponse {
        asset_requests: records
            .into_iter()
            .map(AssetRequestResponse::from)
            .collect(),
        limit,
        offset,
    })
}

pub async fn update_asset_request_status(
    state: &AppState,
    actor_user_id: Uuid,
    request_id: &str,
    payload: AdminUpdateAssetRequestStatusRequest,
) -> Result<AssetRequestResponse, AuthError> {
    let request_id = parse_request_id(request_id)?;
    let record = crud::get_asset_request(&state.db, request_id)
        .await?
        .ok_or_else(|| AuthError::not_found("asset request not found"))?;
    let next_status = parse_status(&payload.status)?;
    let review_notes = match payload.review_notes {
        Some(notes) => normalize_optional_text(Some(notes.as_str()), MAX_NOTES_LEN)?,
        None => record.review_notes.clone(),
    };

    if next_status == AssetRequestStatus::Rejected && review_notes.is_none() {
        return Err(AuthError::bad_request(
            "review_notes are required when rejecting an asset request",
        ));
    }

    if next_status == AssetRequestStatus::Deployed {
        let asset = resolve_existing_asset_for_request(state, &record).await?;
        let deployed = crud::mark_asset_request_deployed(
            &state.db,
            record.id,
            &asset.asset_address,
            asset.last_tx_hash.as_deref(),
            actor_user_id,
            Utc::now(),
        )
        .await?;
        return Ok(AssetRequestResponse::from(deployed));
    }

    let current_status = parse_status(&record.status)?;
    ensure_status_transition(current_status, next_status)?;

    let updated = crud::update_asset_request_status(
        &state.db,
        record.id,
        next_status.as_str(),
        review_notes.as_deref(),
        actor_user_id,
        Utc::now(),
    )
    .await?;

    Ok(AssetRequestResponse::from(updated))
}

pub async fn deploy_asset_request(
    state: &AppState,
    actor_user_id: Uuid,
    request_id: &str,
) -> Result<AssetRequestDeployResponse, AuthError> {
    let request_id = parse_request_id(request_id)?;
    let record = crud::get_asset_request(&state.db, request_id)
        .await?
        .ok_or_else(|| AuthError::not_found("asset request not found"))?;
    let status = parse_status(&record.status)?;

    match status {
        AssetRequestStatus::Approved | AssetRequestStatus::Deployed => {}
        AssetRequestStatus::Submitted => {
            return Err(AuthError::bad_request(
                "asset request must be approved before deployment",
            ));
        }
        AssetRequestStatus::UnderReview => {
            return Err(AuthError::bad_request(
                "asset request is still under review and cannot be deployed",
            ));
        }
        AssetRequestStatus::Rejected => {
            return Err(AuthError::bad_request(
                "rejected asset requests cannot be deployed",
            ));
        }
    }

    if let Some(existing_asset) = find_existing_asset_by_request(state, &record).await? {
        let updated = crud::mark_asset_request_deployed(
            &state.db,
            record.id,
            &existing_asset.asset_address,
            existing_asset.last_tx_hash.as_deref(),
            actor_user_id,
            Utc::now(),
        )
        .await?;

        return Ok(AssetRequestDeployResponse {
            tx_hash: existing_asset.last_tx_hash.clone(),
            request: AssetRequestResponse::from(updated),
            asset: existing_asset,
        });
    }

    let asset_payload = build_asset_creation_request(&record);
    let deployed_asset = asset::create_asset(state, actor_user_id, asset_payload).await?;
    let updated = crud::mark_asset_request_deployed(
        &state.db,
        record.id,
        &deployed_asset.asset.asset_address,
        Some(&deployed_asset.tx_hash),
        actor_user_id,
        Utc::now(),
    )
    .await?;

    Ok(AssetRequestDeployResponse {
        tx_hash: Some(deployed_asset.tx_hash),
        request: AssetRequestResponse::from(updated),
        asset: deployed_asset.asset,
    })
}

fn normalize_new_request(
    user_id: Uuid,
    payload: CreateAssetRequestRequest,
) -> Result<NewAssetRequestRecord, AuthError> {
    let issuer_name = normalize_required_text(&payload.issuer_name, "issuer_name", 160)?;
    let contact_name = normalize_required_text(&payload.contact_name, "contact_name", 120)?;
    let contact_email = normalize_email(&payload.contact_email)?;
    let issuer_website =
        normalize_optional_http_url(payload.issuer_website.as_deref(), "issuer_website")?;
    let issuer_country = normalize_optional_text(payload.issuer_country.as_deref(), 120)?;
    let asset_name = normalize_required_text(&payload.asset_name, "asset_name", 160)?;
    let asset_type_id = normalize_asset_type_id(&payload.asset_type_id)?;
    let description = normalize_required_text(&payload.description, "description", 4_000)?;
    let target_raise = normalize_optional_text(payload.target_raise.as_deref(), 80)?;
    let currency = normalize_optional_currency(payload.currency.as_deref())?;
    let maturity_date = normalize_optional_date(payload.maturity_date.as_deref())?;
    let expected_yield_bps = normalize_optional_yield_bps(payload.expected_yield_bps)?;
    let redemption_summary = normalize_optional_text(payload.redemption_summary.as_deref(), 1_000)?;
    let valuation_source = normalize_optional_text(payload.valuation_source.as_deref(), 500)?;
    let document_urls = normalize_reference_urls(payload.document_urls, "document_urls", MAX_URLS)?;
    let token_symbol = normalize_token_symbol(&payload.token_symbol)?;
    let max_supply = normalize_positive_amount(&payload.max_supply, "max_supply")?;
    let subscription_price =
        normalize_positive_amount(&payload.subscription_price, "subscription_price")?;
    let redemption_price =
        normalize_positive_amount(&payload.redemption_price, "redemption_price")?;
    let metadata_hash = normalize_optional_metadata_hash(payload.metadata_hash.as_deref())?;
    let slug = normalize_optional_slug(payload.slug.as_deref())?;
    let image_url = normalize_optional_reference_url(payload.image_url.as_deref(), "image_url")?;
    let market_segment = normalize_optional_text(payload.market_segment.as_deref(), 120)?;
    let suggested_internal_tags = normalize_string_list(
        payload.suggested_internal_tags,
        "suggested_internal_tags",
        MAX_TAGS,
        40,
    )?;
    let source_urls = normalize_reference_urls(payload.source_urls, "source_urls", MAX_URLS)?;

    Ok(NewAssetRequestRecord {
        submitted_by_user_id: user_id,
        issuer_name,
        contact_name,
        contact_email,
        issuer_website,
        issuer_country,
        asset_name,
        asset_type_id,
        description,
        target_raise,
        currency,
        maturity_date,
        expected_yield_bps,
        redemption_summary,
        valuation_source,
        document_urls,
        token_symbol,
        max_supply,
        subscription_price,
        redemption_price,
        self_service_purchase_enabled: payload.self_service_purchase_enabled,
        metadata_hash,
        slug,
        image_url,
        market_segment,
        suggested_internal_tags,
        source_urls,
    })
}

fn normalize_list_query(
    query: ListAssetRequestsQuery,
) -> Result<(Option<String>, i64, i64), AuthError> {
    let status = query
        .status
        .as_deref()
        .map(parse_status)
        .transpose()?
        .map(|status| status.as_str().to_owned());
    let limit = query.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let offset = query.offset.unwrap_or(0);

    if limit <= 0 || limit > MAX_LIST_LIMIT {
        return Err(AuthError::bad_request(format!(
            "limit must be between 1 and {MAX_LIST_LIMIT}"
        )));
    }
    if offset < 0 {
        return Err(AuthError::bad_request("offset cannot be negative"));
    }

    Ok((status, limit, offset))
}

async fn ensure_request_access(
    state: &AppState,
    user_id: Uuid,
    record: &AssetRequestRecord,
) -> Result<(), AuthError> {
    if record.submitted_by_user_id == user_id || is_admin_user(state, user_id).await? {
        return Ok(());
    }

    Err(AuthError::forbidden(
        "you do not have access to this asset request",
    ))
}

async fn is_admin_user(state: &AppState, user_id: Uuid) -> Result<bool, AuthError> {
    let Some(wallet) = auth_crud::get_wallet_for_user(&state.db, user_id).await? else {
        return Ok(false);
    };

    Ok(state.env.is_admin_wallet(&wallet.wallet_address))
}

async fn find_existing_asset_by_request(
    state: &AppState,
    request: &AssetRequestRecord,
) -> Result<Option<crate::module::asset::schema::AssetResponse>, AuthError> {
    match asset::get_asset_by_proposal(state, &request.proposal_id.to_string()).await {
        Ok(asset) => Ok(Some(asset)),
        Err(error) if error.is_not_found() => Ok(None),
        Err(error) => Err(error),
    }
}

async fn resolve_existing_asset_for_request(
    state: &AppState,
    request: &AssetRequestRecord,
) -> Result<crate::module::asset::schema::AssetResponse, AuthError> {
    find_existing_asset_by_request(state, request)
        .await?
        .ok_or_else(|| {
            AuthError::bad_request(
                "cannot set status to deployed before an asset exists; use the deploy endpoint",
            )
        })
}

fn build_asset_creation_request(record: &AssetRequestRecord) -> AdminCreateAssetRequest {
    AdminCreateAssetRequest {
        proposal_id: record.proposal_id.to_string(),
        asset_type_id: record.asset_type_id.clone(),
        name: record.asset_name.clone(),
        symbol: record.token_symbol.clone(),
        max_supply: record.max_supply.clone(),
        subscription_price: record.subscription_price.clone(),
        redemption_price: record.redemption_price.clone(),
        self_service_purchase_enabled: record.self_service_purchase_enabled,
        metadata_hash: record.metadata_hash.clone(),
        slug: record.slug.clone(),
        image_url: record.image_url.clone(),
        summary: Some(record.description.clone()),
        market_segment: record.market_segment.clone(),
        suggested_internal_tags: record.suggested_internal_tags.clone(),
        sources: record.source_urls.clone(),
        featured: false,
        visible: true,
        searchable: true,
    }
}

fn parse_request_id(raw: &str) -> Result<Uuid, AuthError> {
    Uuid::parse_str(raw.trim()).map_err(|_| AuthError::bad_request("invalid asset request id"))
}

fn normalize_required_text(
    raw: &str,
    field_name: &str,
    max_len: usize,
) -> Result<String, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }
    if value.len() > max_len {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be {max_len} characters or fewer"
        )));
    }

    Ok(value.to_owned())
}

fn normalize_optional_text(raw: Option<&str>, max_len: usize) -> Result<Option<String>, AuthError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    if value.len() > max_len {
        return Err(AuthError::bad_request(format!(
            "value must be {max_len} characters or fewer"
        )));
    }

    Ok(Some(value.to_owned()))
}

fn normalize_email(raw: &str) -> Result<String, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request("contact_email is required"));
    }
    if value.len() > 320 || value.contains(char::is_whitespace) {
        return Err(AuthError::bad_request("invalid contact_email"));
    }

    let Some((local, domain)) = value.split_once('@') else {
        return Err(AuthError::bad_request("invalid contact_email"));
    };
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(AuthError::bad_request("invalid contact_email"));
    }

    Ok(value.to_ascii_lowercase())
}

fn normalize_optional_http_url(
    raw: Option<&str>,
    field_name: &str,
) -> Result<Option<String>, AuthError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let parsed = Url::parse(value)
        .map_err(|_| AuthError::bad_request(format!("invalid {field_name} url")))?;
    match parsed.scheme() {
        "http" | "https" => Ok(Some(value.to_owned())),
        _ => Err(AuthError::bad_request(format!(
            "{field_name} must use http or https"
        ))),
    }
}

fn normalize_optional_reference_url(
    raw: Option<&str>,
    field_name: &str,
) -> Result<Option<String>, AuthError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    validate_reference_url(value, field_name)?;
    Ok(Some(value.to_owned()))
}

fn normalize_reference_urls(
    values: Vec<String>,
    field_name: &str,
    max_items: usize,
) -> Result<Vec<String>, AuthError> {
    normalize_string_list_with_validator(values, field_name, max_items, 2048, |value| {
        validate_reference_url(value, field_name)
    })
}

fn validate_reference_url(value: &str, field_name: &str) -> Result<(), AuthError> {
    if value.starts_with("ipfs://") {
        if value.len() <= "ipfs://".len() {
            return Err(AuthError::bad_request(format!(
                "invalid {field_name} entry"
            )));
        }
        return Ok(());
    }

    let parsed = Url::parse(value)
        .map_err(|_| AuthError::bad_request(format!("invalid {field_name} entry")))?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        _ => Err(AuthError::bad_request(format!(
            "{field_name} entries must use http, https, or ipfs"
        ))),
    }
}

fn normalize_optional_currency(raw: Option<&str>) -> Result<Option<String>, AuthError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > 16
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(AuthError::bad_request(
            "currency must be 16 characters or fewer and use letters, digits, - or _",
        ));
    }

    Ok(Some(value.to_ascii_uppercase()))
}

fn normalize_optional_date(raw: Option<&str>) -> Result<Option<NaiveDate>, AuthError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| AuthError::bad_request("maturity_date must use YYYY-MM-DD"))
}

fn normalize_optional_yield_bps(value: Option<i32>) -> Result<Option<i32>, AuthError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if !(0..=1_000_000).contains(&value) {
        return Err(AuthError::bad_request(
            "expected_yield_bps must be between 0 and 1000000",
        ));
    }

    Ok(Some(value))
}

fn normalize_asset_type_id(raw: &str) -> Result<String, AuthError> {
    let value = normalize_required_text(raw, "asset_type_id", 66)?;
    parse_bytes32_input(&value, "asset_type_id")?;
    Ok(value)
}

fn normalize_token_symbol(raw: &str) -> Result<String, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request("token_symbol is required"));
    }
    if value.len() > 16
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-')
    {
        return Err(AuthError::bad_request(
            "token_symbol must be 16 characters or fewer and use letters, digits, ., _, or -",
        ));
    }

    Ok(value.to_ascii_uppercase())
}

fn normalize_positive_amount(raw: &str, field_name: &str) -> Result<String, AuthError> {
    let value = parse_u256(raw, field_name)?;
    if value.is_zero() {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be greater than zero"
        )));
    }

    Ok(value.to_string())
}

fn normalize_optional_metadata_hash(raw: Option<&str>) -> Result<Option<String>, AuthError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    parse_bytes32_input(value, "metadata_hash")?;
    Ok(Some(value.to_owned()))
}

fn normalize_optional_slug(raw: Option<&str>) -> Result<Option<String>, AuthError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > 120
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(AuthError::bad_request(
            "slug must be lowercase and may only include letters, digits, and hyphens",
        ));
    }

    Ok(Some(value.to_owned()))
}

fn normalize_string_list(
    values: Vec<String>,
    field_name: &str,
    max_items: usize,
    max_len: usize,
) -> Result<Vec<String>, AuthError> {
    normalize_string_list_with_validator(values, field_name, max_items, max_len, |_| Ok(()))
}

fn normalize_string_list_with_validator<F>(
    values: Vec<String>,
    field_name: &str,
    max_items: usize,
    max_len: usize,
    mut validator: F,
) -> Result<Vec<String>, AuthError>
where
    F: FnMut(&str) -> Result<(), AuthError>,
{
    if values.len() > max_items {
        return Err(AuthError::bad_request(format!(
            "{field_name} cannot contain more than {max_items} items"
        )));
    }

    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.len() > max_len {
            return Err(AuthError::bad_request(format!(
                "{field_name} entries must be {max_len} characters or fewer"
            )));
        }
        validator(trimmed)?;

        if !normalized.iter().any(|existing| existing == trimmed) {
            normalized.push(trimmed.to_owned());
        }
    }

    Ok(normalized)
}

fn parse_status(raw: &str) -> Result<AssetRequestStatus, AuthError> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "submitted" => Ok(AssetRequestStatus::Submitted),
        "under_review" => Ok(AssetRequestStatus::UnderReview),
        "approved" => Ok(AssetRequestStatus::Approved),
        "rejected" => Ok(AssetRequestStatus::Rejected),
        "deployed" => Ok(AssetRequestStatus::Deployed),
        _ => Err(AuthError::bad_request(
            "invalid asset request status, expected submitted|under_review|approved|rejected|deployed",
        )),
    }
}

fn ensure_status_transition(
    current: AssetRequestStatus,
    next: AssetRequestStatus,
) -> Result<(), AuthError> {
    let allowed = match current {
        AssetRequestStatus::Submitted => matches!(
            next,
            AssetRequestStatus::Submitted
                | AssetRequestStatus::UnderReview
                | AssetRequestStatus::Approved
                | AssetRequestStatus::Rejected
        ),
        AssetRequestStatus::UnderReview => matches!(
            next,
            AssetRequestStatus::UnderReview
                | AssetRequestStatus::Approved
                | AssetRequestStatus::Rejected
        ),
        AssetRequestStatus::Approved => matches!(
            next,
            AssetRequestStatus::UnderReview
                | AssetRequestStatus::Approved
                | AssetRequestStatus::Rejected
        ),
        AssetRequestStatus::Rejected => matches!(
            next,
            AssetRequestStatus::Rejected | AssetRequestStatus::UnderReview
        ),
        AssetRequestStatus::Deployed => matches!(next, AssetRequestStatus::Deployed),
    };

    if allowed {
        return Ok(());
    }

    Err(AuthError::bad_request(format!(
        "cannot change asset request status from {} to {}",
        current.as_str(),
        next.as_str()
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetRequestStatus {
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Deployed,
}

impl AssetRequestStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::UnderReview => "under_review",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Deployed => "deployed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AssetRequestStatus, ensure_status_transition, normalize_email,
        normalize_optional_reference_url, normalize_optional_slug, normalize_positive_amount,
        normalize_token_symbol, parse_status,
    };

    #[test]
    fn parses_asset_request_status() {
        assert_eq!(
            parse_status("submitted").unwrap(),
            AssetRequestStatus::Submitted
        );
        assert!(parse_status("bad").is_err());
    }

    #[test]
    fn rejects_invalid_deployed_transition() {
        let error =
            ensure_status_transition(AssetRequestStatus::Submitted, AssetRequestStatus::Deployed)
                .expect_err("transition should fail");
        assert!(
            error
                .to_string()
                .contains("cannot change asset request status")
        );
    }

    #[test]
    fn normalizes_core_submission_fields() {
        assert_eq!(
            normalize_email("Issuer@Example.com").unwrap(),
            "issuer@example.com"
        );
        assert_eq!(normalize_token_symbol("fgn2031").unwrap(), "FGN2031");
        assert_eq!(
            normalize_optional_slug(Some("fgn-2031")).unwrap(),
            Some("fgn-2031".to_owned())
        );
        assert_eq!(
            normalize_positive_amount("1017225", "subscription_price").unwrap(),
            "1017225"
        );
    }

    #[test]
    fn accepts_reference_urls_used_by_asset_requests() {
        assert_eq!(
            normalize_optional_reference_url(Some("https://example.com/doc.pdf"), "document_url")
                .unwrap(),
            Some("https://example.com/doc.pdf".to_owned())
        );
        assert_eq!(
            normalize_optional_reference_url(Some("ipfs://bafybeigdyrzt"), "document_url").unwrap(),
            Some("ipfs://bafybeigdyrzt".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_submission_inputs() {
        assert!(normalize_email("issuer").is_err());
        assert!(normalize_token_symbol("bad symbol").is_err());
        assert!(normalize_optional_slug(Some("FGN2031")).is_err());
        assert!(normalize_positive_amount("0", "subscription_price").is_err());
        assert!(
            normalize_optional_reference_url(Some("ftp://example.com/doc.pdf"), "document_url")
                .is_err()
        );
    }
}
