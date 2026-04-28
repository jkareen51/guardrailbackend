use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::auth::error::AuthError,
    module::auth::model::{
        NewWalletRecord, SmartAccountSignerRecord, UserProfileRecord, UserRecord,
        VerifiedGoogleToken, WalletChallengeRecord, WalletRecord,
    },
};

mod sql {
    pub const FIND_USER_BY_GOOGLE_SUB: &str = include_str!("sql/find_user_by_google_sub.sql");
    pub const FIND_USER_BY_WALLET_ADDRESS: &str =
        include_str!("sql/find_user_by_wallet_address.sql");
    pub const GET_USER_BY_ID: &str = include_str!("sql/get_user_by_id.sql");
    pub const GET_WALLET_FOR_USER: &str = include_str!("sql/get_wallet_for_user.sql");
    pub const GET_SMART_ACCOUNT_SIGNER_FOR_USER: &str = r#"
        SELECT
            wallet_address,
            owner_address,
            owner_provider,
            owner_ref,
            factory_address,
            entry_point_address,
            owner_encrypted_private_key,
            owner_encryption_nonce
        FROM wallet_accounts
        WHERE user_id = $1
          AND account_kind = 'smart_account'
          AND owner_address IS NOT NULL
          AND owner_provider IS NOT NULL
          AND owner_ref IS NOT NULL
          AND factory_address IS NOT NULL
          AND entry_point_address IS NOT NULL
          AND owner_encrypted_private_key IS NOT NULL
          AND owner_encryption_nonce IS NOT NULL
    "#;
    pub const GET_USER_PROFILE_BY_ID: &str = r#"
        SELECT
            u.id,
            u.email,
            u.username,
            u.display_name,
            u.avatar_url,
            u.created_at,
            u.updated_at,
            w.wallet_address,
            w.chain_id AS wallet_chain_id,
            w.account_kind AS wallet_account_kind,
            w.owner_address AS wallet_owner_address,
            w.owner_provider AS wallet_owner_provider,
            w.factory_address AS wallet_factory_address,
            w.entry_point_address AS wallet_entry_point_address,
            w.created_at AS wallet_created_at
        FROM users u
        LEFT JOIN wallet_accounts w ON w.user_id = u.id
        WHERE u.id = $1
    "#;
    pub const GET_WALLET_CHALLENGE_BY_ID: &str = include_str!("sql/get_wallet_challenge_by_id.sql");
    pub const UPDATE_GOOGLE_USER: &str = include_str!("sql/update_google_user.sql");
    pub const UPDATE_GOOGLE_IDENTITY: &str = include_str!("sql/update_google_identity.sql");
    pub const INSERT_USER: &str = include_str!("sql/insert_user.sql");
    pub const INSERT_GOOGLE_IDENTITY: &str = include_str!("sql/insert_google_identity.sql");
    pub const INSERT_MANAGED_WALLET: &str = include_str!("sql/insert_managed_wallet.sql");
    pub const INSERT_WALLET_CHALLENGE: &str = include_str!("sql/insert_wallet_challenge.sql");
    pub const CONSUME_WALLET_CHALLENGE: &str = include_str!("sql/consume_wallet_challenge.sql");
    pub const INSERT_WALLET_USER: &str = include_str!("sql/insert_wallet_user.sql");
    pub const INSERT_WALLET_ACCOUNT: &str = include_str!("sql/insert_wallet_account.sql");
}

pub async fn get_user_by_id(pool: &DbPool, user_id: Uuid) -> Result<Option<UserRecord>, AuthError> {
    sqlx::query_as::<_, UserRecord>(sql::GET_USER_BY_ID)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_wallet_for_user(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Option<WalletRecord>, AuthError> {
    sqlx::query_as::<_, WalletRecord>(sql::GET_WALLET_FOR_USER)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_smart_account_signer_for_user(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Option<SmartAccountSignerRecord>, AuthError> {
    sqlx::query_as::<_, SmartAccountSignerRecord>(sql::GET_SMART_ACCOUNT_SIGNER_FOR_USER)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_user_profile_by_id(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Option<UserProfileRecord>, AuthError> {
    sqlx::query_as::<_, UserProfileRecord>(sql::GET_USER_PROFILE_BY_ID)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_wallet_challenge_by_id(
    pool: &DbPool,
    challenge_id: Uuid,
) -> Result<Option<WalletChallengeRecord>, AuthError> {
    sqlx::query_as::<_, WalletChallengeRecord>(sql::GET_WALLET_CHALLENGE_BY_ID)
        .bind(challenge_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_google_user(
    pool: &DbPool,
    token: &VerifiedGoogleToken,
) -> Result<UserRecord, AuthError> {
    let mut tx = pool.begin().await?;

    if let Some(existing_user) = find_user_by_google_sub_tx(&mut tx, &token.google_sub).await? {
        let updated_user = sqlx::query_as::<_, UserRecord>(sql::UPDATE_GOOGLE_USER)
            .bind(existing_user.id)
            .bind(token.email.as_deref())
            .bind(token.display_name.as_deref())
            .bind(token.avatar_url.as_deref())
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query(sql::UPDATE_GOOGLE_IDENTITY)
            .bind(existing_user.id)
            .bind(token.email.as_deref())
            .bind(token.email_verified)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        return Ok(updated_user);
    }

    let user_id = Uuid::new_v4();
    let inserted_user = sqlx::query_as::<_, UserRecord>(sql::INSERT_USER)
        .bind(user_id)
        .bind(token.email.as_deref())
        .bind(Option::<&str>::None)
        .bind(token.display_name.as_deref())
        .bind(token.avatar_url.as_deref())
        .fetch_one(&mut *tx)
        .await?;

    let identity_result = sqlx::query(sql::INSERT_GOOGLE_IDENTITY)
        .bind(user_id)
        .bind(&token.google_sub)
        .bind(token.email.as_deref())
        .bind(token.email_verified)
        .execute(&mut *tx)
        .await;

    match identity_result {
        Ok(_) => {
            tx.commit().await?;
            Ok(inserted_user)
        }
        Err(error) if is_unique_violation(&error) => {
            tx.rollback().await?;
            find_user_by_google_sub(pool, &token.google_sub)
                .await?
                .ok_or_else(|| AuthError::unauthorized("user not found"))
        }
        Err(error) => Err(AuthError::from(error)),
    }
}

pub async fn upsert_wallet_account(
    pool: &DbPool,
    user_id: Uuid,
    wallet: &NewWalletRecord,
) -> Result<WalletRecord, AuthError> {
    let result = sqlx::query_as::<_, WalletRecord>(sql::INSERT_MANAGED_WALLET)
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&wallet.wallet_address)
        .bind(wallet.chain_id)
        .bind(&wallet.account_kind)
        .bind(wallet.owner_address.as_deref())
        .bind(wallet.owner_provider.as_deref())
        .bind(wallet.owner_ref.as_deref())
        .bind(wallet.factory_address.as_deref())
        .bind(wallet.entry_point_address.as_deref())
        .bind(wallet.owner_encrypted_private_key.as_deref())
        .bind(wallet.owner_encryption_nonce.as_deref())
        .bind(wallet.owner_key_version)
        .fetch_one(pool)
        .await;

    match result {
        Ok(wallet) => Ok(wallet),
        Err(error) if is_unique_violation(&error) => {
            Err(AuthError::conflict("wallet already linked to another user"))
        }
        Err(error) => Err(AuthError::from(error)),
    }
}

pub async fn create_wallet_challenge(
    pool: &DbPool,
    challenge_id: Uuid,
    wallet_address: &str,
    chain_id: i64,
    nonce: &str,
    message: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<WalletChallengeRecord, AuthError> {
    sqlx::query_as::<_, WalletChallengeRecord>(sql::INSERT_WALLET_CHALLENGE)
        .bind(challenge_id)
        .bind(wallet_address)
        .bind(chain_id)
        .bind(nonce)
        .bind(message)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn consume_wallet_challenge(
    pool: &DbPool,
    challenge_id: Uuid,
) -> Result<bool, AuthError> {
    let result = sqlx::query(sql::CONSUME_WALLET_CHALLENGE)
        .bind(challenge_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn find_user_by_wallet_address(
    pool: &DbPool,
    wallet_address: &str,
) -> Result<Option<UserRecord>, AuthError> {
    sqlx::query_as::<_, UserRecord>(sql::FIND_USER_BY_WALLET_ADDRESS)
        .bind(wallet_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn create_wallet_user(
    pool: &DbPool,
    username: &str,
    wallet_address: &str,
    chain_id: i64,
) -> Result<UserRecord, AuthError> {
    let mut tx = pool.begin().await?;
    let user_id = Uuid::new_v4();

    let inserted_user = sqlx::query_as::<_, UserRecord>(sql::INSERT_WALLET_USER)
        .bind(user_id)
        .bind(username)
        .bind(username)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_unique_user_error)?;

    let wallet = NewWalletRecord::external_eoa(wallet_address.to_owned(), chain_id);
    let wallet_insert = sqlx::query_as::<_, WalletRecord>(sql::INSERT_WALLET_ACCOUNT)
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&wallet.wallet_address)
        .bind(wallet.chain_id)
        .bind(&wallet.account_kind)
        .bind(wallet.owner_address.as_deref())
        .bind(wallet.owner_provider.as_deref())
        .bind(wallet.owner_ref.as_deref())
        .bind(wallet.factory_address.as_deref())
        .bind(wallet.entry_point_address.as_deref())
        .bind(wallet.owner_encrypted_private_key.as_deref())
        .bind(wallet.owner_encryption_nonce.as_deref())
        .bind(wallet.owner_key_version)
        .fetch_one(&mut *tx)
        .await;

    match wallet_insert {
        Ok(_) => {
            tx.commit().await?;
            Ok(inserted_user)
        }
        Err(error) if unique_constraint(&error) == Some("wallet_accounts_address_key") => {
            tx.rollback().await?;
            find_user_by_wallet_address(pool, wallet_address)
                .await?
                .ok_or_else(|| AuthError::conflict("wallet already linked to another user"))
        }
        Err(error) => Err(AuthError::from(error)),
    }
}

async fn find_user_by_google_sub(
    pool: &DbPool,
    google_sub: &str,
) -> Result<Option<UserRecord>, AuthError> {
    sqlx::query_as::<_, UserRecord>(sql::FIND_USER_BY_GOOGLE_SUB)
        .bind(google_sub)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

async fn find_user_by_google_sub_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    google_sub: &str,
) -> Result<Option<UserRecord>, AuthError> {
    sqlx::query_as::<_, UserRecord>(sql::FIND_USER_BY_GOOGLE_SUB)
        .bind(google_sub)
        .fetch_optional(&mut **tx)
        .await
        .map_err(AuthError::from)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505")
    )
}

fn unique_constraint(error: &sqlx::Error) -> Option<&str> {
    match error {
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505") =>
        {
            database_error.constraint()
        }
        _ => None,
    }
}

fn map_unique_user_error(error: sqlx::Error) -> AuthError {
    match unique_constraint(&error) {
        Some("users_username_key") => AuthError::conflict("username already taken"),
        _ => AuthError::from(error),
    }
}
