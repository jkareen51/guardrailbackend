use axum::http::{HeaderMap, header};
use chrono::{Duration, Utc};
use ethers_core::types::{Address, Signature};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::auth::{
        crud,
        error::AuthError,
        model::{
            ACCOUNT_KIND_SMART_ACCOUNT, UserRecord, VerifiedGoogleToken, WalletChallengeRecord,
        },
        schema::{
            AuthResponse, GoogleSignInRequest, MeResponse, UserResponse, WalletChallengeRequest,
            WalletChallengeResponse, WalletConnectRequest,
        },
    },
    service::{
        aa::provision_local_smart_account,
        jwt::{AuthenticatedUser, create_session_token},
    },
};

const APP_NAME: &str = "Sabi";
const WALLET_CHALLENGE_TTL_MINUTES: i64 = 10;

pub async fn sign_in_with_google(
    state: &AppState,
    headers: &HeaderMap,
    payload: GoogleSignInRequest,
) -> Result<AuthResponse, AuthError> {
    validate_google_csrf(headers, &payload)?;

    if let Some(client_id) = payload.client_id.as_deref() {
        if client_id != state.env.google_client_id {
            return Err(AuthError::bad_request("unexpected google client id"));
        }
    }

    let verified = verify_google_id_token(state, &payload.credential).await?;
    let user = crud::upsert_google_user(&state.db, &verified).await?;
    let user = ensure_google_wallet(state, user).await?;

    build_auth_response(state, user).await
}

pub async fn create_wallet_challenge(
    state: &AppState,
    payload: WalletChallengeRequest,
) -> Result<WalletChallengeResponse, AuthError> {
    let wallet_address = normalize_wallet_address(&payload.wallet_address)?;
    issue_wallet_challenge(state, &wallet_address).await
}

pub(crate) async fn issue_wallet_challenge(
    state: &AppState,
    wallet_address: &str,
) -> Result<WalletChallengeResponse, AuthError> {
    let challenge_id = Uuid::new_v4();
    let nonce = Uuid::new_v4().simple().to_string();
    let expires_at = Utc::now() + Duration::minutes(WALLET_CHALLENGE_TTL_MINUTES);
    let message = build_wallet_challenge_message(wallet_address, state.env.monad_chain_id, &nonce);

    let challenge = crud::create_wallet_challenge(
        &state.db,
        challenge_id,
        wallet_address,
        state.env.monad_chain_id,
        &nonce,
        &message,
        expires_at,
    )
    .await?;

    Ok(WalletChallengeResponse {
        challenge_id: challenge.id,
        message: challenge.message,
        expires_at: challenge.expires_at,
    })
}

pub async fn connect_wallet(
    state: &AppState,
    payload: WalletConnectRequest,
) -> Result<AuthResponse, AuthError> {
    let challenge = load_active_wallet_challenge(state, payload.challenge_id).await?;
    complete_wallet_connection(
        state,
        challenge,
        payload.username.as_deref(),
        &payload.signature,
    )
    .await
}

pub async fn get_me(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> Result<MeResponse, AuthError> {
    let profile = crud::get_user_profile_by_id(&state.db, authenticated_user.user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("invalid session"))?;
    let (user, wallet) = profile.into_parts();

    Ok(MeResponse {
        user: UserResponse::from_parts(user, wallet),
    })
}

pub fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;

    cookies
        .split(';')
        .filter_map(|part| part.split_once('='))
        .find_map(|(key, value)| {
            let key = key.trim();
            let value = value.trim();
            (key == name).then(|| value.to_owned())
        })
}

pub fn normalize_wallet_address(raw: &str) -> Result<String, AuthError> {
    let address: Address = raw
        .parse()
        .map_err(|_| AuthError::bad_request("invalid wallet address"))?;

    Ok(format!("{address:#x}"))
}

pub fn normalize_username(raw: &str) -> Result<String, AuthError> {
    let username = raw.trim().to_ascii_lowercase();

    if !(3..=24).contains(&username.len()) {
        return Err(AuthError::bad_request(
            "username must be between 3 and 24 characters",
        ));
    }

    if !username
        .chars()
        .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == '_')
    {
        return Err(AuthError::bad_request(
            "username can only contain lowercase letters, numbers, and underscores",
        ));
    }

    Ok(username)
}

async fn build_auth_response(
    state: &AppState,
    user: UserRecord,
) -> Result<AuthResponse, AuthError> {
    let wallet = crud::get_wallet_for_user(&state.db, user.id).await?;
    let token = create_session_token(&state.env, &user)
        .map_err(|error| AuthError::internal("jwt encoding failed", error))?;

    Ok(AuthResponse {
        token,
        user: UserResponse::from_parts(user, wallet),
    })
}

pub(crate) async fn load_active_wallet_challenge(
    state: &AppState,
    challenge_id: Uuid,
) -> Result<WalletChallengeRecord, AuthError> {
    let challenge = crud::get_wallet_challenge_by_id(&state.db, challenge_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("invalid wallet challenge"))?;

    validate_wallet_challenge(&challenge)?;

    Ok(challenge)
}

pub(crate) async fn complete_wallet_connection(
    state: &AppState,
    challenge: WalletChallengeRecord,
    username: Option<&str>,
    raw_signature: &str,
) -> Result<AuthResponse, AuthError> {
    verify_wallet_signature(&challenge, raw_signature)?;

    if let Some(user) =
        crud::find_user_by_wallet_address(&state.db, &challenge.wallet_address).await?
    {
        if !crud::consume_wallet_challenge(&state.db, challenge.id).await? {
            return Err(AuthError::unauthorized("invalid wallet challenge"));
        }

        return build_auth_response(state, user).await;
    }

    let username = username
        .ok_or_else(|| AuthError::bad_request("username is required for new wallet users"))?;
    let username = normalize_username(username)?;

    if !crud::consume_wallet_challenge(&state.db, challenge.id).await? {
        return Err(AuthError::unauthorized("invalid wallet challenge"));
    }

    let user = crud::create_wallet_user(
        &state.db,
        &username,
        &challenge.wallet_address,
        challenge.chain_id,
    )
    .await?;

    build_auth_response(state, user).await
}

async fn ensure_google_wallet(state: &AppState, user: UserRecord) -> Result<UserRecord, AuthError> {
    if let Some(existing_wallet) = crud::get_wallet_for_user(&state.db, user.id).await? {
        if existing_wallet.account_kind == ACCOUNT_KIND_SMART_ACCOUNT {
            return Ok(user);
        }
    }

    let wallet = provision_local_smart_account(&state.env, user.id)
        .await
        .map_err(|error| AuthError::internal("smart-account provisioning failed", error))?;

    match crud::upsert_wallet_account(&state.db, user.id, &wallet).await {
        Ok(_) => Ok(user),
        Err(error) if error.is_conflict() => {
            let existing_wallet = crud::get_wallet_for_user(&state.db, user.id).await?;

            if matches!(existing_wallet, Some(wallet) if wallet.account_kind == ACCOUNT_KIND_SMART_ACCOUNT)
            {
                Ok(user)
            } else {
                Err(error)
            }
        }
        Err(error) => Err(error),
    }
}

fn validate_wallet_challenge(challenge: &WalletChallengeRecord) -> Result<(), AuthError> {
    if challenge.consumed_at.is_some() || challenge.expires_at <= Utc::now() {
        return Err(AuthError::unauthorized("invalid wallet challenge"));
    }

    Ok(())
}

fn verify_wallet_signature(
    challenge: &WalletChallengeRecord,
    raw_signature: &str,
) -> Result<(), AuthError> {
    let expected_address: Address = challenge
        .wallet_address
        .parse()
        .map_err(|error| AuthError::internal("invalid stored wallet address", error))?;
    let signature: Signature = raw_signature
        .parse()
        .map_err(|_| AuthError::bad_request("invalid wallet signature"))?;

    signature
        .verify(challenge.message.as_bytes(), expected_address)
        .map_err(|_| AuthError::unauthorized("wallet signature verification failed"))
}

fn build_wallet_challenge_message(wallet_address: &str, chain_id: i64, nonce: &str) -> String {
    format!(
        "Sign this message to sign in to {APP_NAME}.\n\nWallet: {wallet_address}\nChain ID: {chain_id}\nNonce: {nonce}"
    )
}

async fn verify_google_id_token(
    state: &AppState,
    id_token: &str,
) -> Result<VerifiedGoogleToken, AuthError> {
    let header = decode_header(id_token)
        .map_err(|_| AuthError::unauthorized("invalid google credential header"))?;

    if header.alg != Algorithm::RS256 {
        return Err(AuthError::unauthorized(
            "unsupported google credential algorithm",
        ));
    }

    let key_id = header
        .kid
        .ok_or_else(|| AuthError::unauthorized("google credential is missing key id"))?;
    let jwks = state
        .http_client
        .get(&state.env.google_jwks_url)
        .send()
        .await
        .map_err(|error| AuthError::internal("failed to fetch google jwks", error))?
        .error_for_status()
        .map_err(|error| AuthError::internal("google jwks request failed", error))?
        .json::<GoogleJwks>()
        .await
        .map_err(|error| AuthError::internal("failed to decode google jwks", error))?;

    let jwk = jwks
        .keys
        .into_iter()
        .find(|value| value.kid == key_id)
        .ok_or_else(|| AuthError::unauthorized("google signing key not found"))?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|error| AuthError::internal("failed to build google decoding key", error))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[state.env.google_client_id.as_str()]);
    validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);
    validation.validate_exp = true;

    let claims = decode::<GoogleIdTokenClaims>(id_token, &decoding_key, &validation)
        .map_err(|_| AuthError::unauthorized("invalid google credential"))?
        .claims;

    Ok(VerifiedGoogleToken {
        google_sub: claims.sub,
        email: claims.email,
        email_verified: claims.email_verified.unwrap_or(false),
        display_name: claims.name,
        avatar_url: claims.picture,
    })
}

fn validate_google_csrf(
    headers: &HeaderMap,
    payload: &GoogleSignInRequest,
) -> Result<(), AuthError> {
    let cookie_token = extract_cookie(headers, "g_csrf_token");
    let body_token = payload.g_csrf_token.as_deref();

    match (cookie_token.as_deref(), body_token) {
        (None, None) => Ok(()),
        (Some(cookie), Some(body)) if cookie == body => Ok(()),
        _ => Err(AuthError::unauthorized("invalid google csrf token")),
    }
}

#[derive(Debug, Deserialize)]
struct GoogleJwks {
    keys: Vec<GoogleJwk>,
}

#[derive(Debug, Deserialize)]
struct GoogleJwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Debug, Deserialize)]
struct GoogleIdTokenClaims {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
}
