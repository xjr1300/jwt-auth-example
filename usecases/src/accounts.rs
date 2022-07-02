use secrecy::Secret;
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use configurations::{
    password::{verify_password, AuthError},
    session::{SessionData, TypedSession},
    telemetries::spawn_blocking_with_tracing,
    tokens::generate_jwt_pair,
    Settings, TokensSettings,
};
use domains::models::{
    users::{HashedPassword, RawPassword, User, UserId, UserName},
    EmailAddress,
};
use infrastructures::repositories::users::PgUserRepository;
use miscellaneous::current_unix_epoch;

#[derive(Debug, thiserror::Error)]
pub enum SignupError {
    #[error(transparent)]
    UnexpectedError(anyhow::Error),
    #[error("Eメールアドレスが既に登録されています。")]
    EmailAddressAlreadyExists,
}

#[derive(Debug, Serialize)]
pub struct SignupResult {
    pub id: Uuid,
    pub user_name: String,
    pub email_address: String,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub async fn signup(
    user_name: UserName,
    email_address: EmailAddress,
    password: RawPassword,
    pool: &PgPool,
) -> anyhow::Result<SignupResult, SignupError> {
    // トランザクションを開始
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| SignupError::UnexpectedError(e.into()))?;
    // リポジトリを構築
    let repository = PgUserRepository::default();

    // メールアドレスが一致するユーザーが存在しないか確認
    let found = repository
        .get_by_email_address(&email_address, &mut tx)
        .await
        .map_err(|e| SignupError::UnexpectedError(e.into()))?;
    if found.is_some() {
        return Err(SignupError::EmailAddressAlreadyExists);
    }

    // ユーザーを登録
    let hashed_password = HashedPassword::new(&password).map_err(SignupError::UnexpectedError)?;
    let user = User::new(
        UserId::default(),
        user_name,
        email_address,
        hashed_password,
        true,
        None,
        None,
        None,
    );
    let user = repository
        .insert(&user, &mut tx)
        .await
        .map_err(|e| SignupError::UnexpectedError(e.into()))?;

    // トランザクションをコミット
    tx.commit()
        .await
        .map_err(|e| SignupError::UnexpectedError(e.into()))?;

    Ok(SignupResult {
        id: user.id().value().to_owned(),
        user_name: user.user_name().value().to_owned(),
        email_address: user.email_address().value().to_owned(),
        is_active: user.is_active(),
        created_at: user.created_at().unwrap(),
        updated_at: user.updated_at().unwrap(),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error(transparent)]
    UnexpectedError(anyhow::Error),
    #[error("Eメールアドレスまたはパスワードが異なります。")]
    InvalidCredentials,
    #[error("ユーザー({0})が無効になっています。")]
    NotActive(Uuid),
}

/// データベースからユーザーを取得して、パスワードを検証する。
///
/// # Arguments
///
/// * `email_address` - Eメールアドレス。
/// * `raw_password` - パスワード。
/// * `tx` - トランザクション。
///
/// # Returns
///
/// * ユーザーインスタンス。
#[tracing::instrument(name = "Validate credentials", skip(raw_password, tx))]
async fn validate_credentials(
    email_address: EmailAddress,
    raw_password: Secret<String>,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<User, LoginError> {
    // Eメールアドレスからユーザーを取得
    let result = PgUserRepository::default()
        .get_by_email_address(&email_address, tx)
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    if result.is_none() {
        return Err(LoginError::InvalidCredentials);
    }

    // 引数で受け取ったパスワードをハッシュ化した結果が、ユーザーに記録されているハッシュ化パスワードと一致するか確認
    let user = result.unwrap();
    let expected_hashed = user.hashed_password().value().to_owned();
    let result =
        spawn_blocking_with_tracing(move || verify_password(&expected_hashed, &raw_password))
            .await
            .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    if let Err(e) = result {
        return Err(match e {
            AuthError::InvalidCredentials(_) => LoginError::InvalidCredentials,
            AuthError::UnexpectedError(e) => LoginError::UnexpectedError(e),
        });
    }

    Ok(user)
}

/// セッションデータを生成する。
///
/// # Arguments
///
/// * `user_id` - ユーザーID。
/// * `token_settings` - トークン設定。
///
/// # Returns
///
/// セッションデータ。
fn generate_session_data(
    user_id: Uuid,
    token_settings: &TokensSettings,
) -> Result<SessionData, LoginError> {
    let base_epoch = current_unix_epoch();
    let access_expiration = base_epoch + token_settings.access_token_duration();
    let refresh_expiration = base_epoch + token_settings.refresh_token_duration();
    let (access_token, refresh_token) = generate_jwt_pair(
        user_id,
        &token_settings.secret_key,
        access_expiration,
        refresh_expiration,
    )
    .map_err(LoginError::UnexpectedError)?;

    Ok(SessionData {
        user_id,
        access_token,
        access_expiration,
        refresh_token,
        refresh_expiration,
    })
}

/// ユーザーの最終更新日時を更新する。
///
/// # Arguments
///
/// * `user_id` - 最終更新日時を更新するユーザーのユーザーID。
///
/// # Returns
///
/// `()`。
async fn update_last_logged_in(
    user_id: UserId,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), LoginError> {
    PgUserRepository::default()
        .update_last_logged_in(user_id, tx)
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    Ok(())
}

pub async fn login(
    email_address: EmailAddress,
    raw_password: Secret<String>,
    settings: &Settings,
    session: &TypedSession,
    pool: &PgPool,
) -> anyhow::Result<SessionData, LoginError> {
    // トランザクションを開始
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // データベースからユーザーを取得して、パスワードを検証
    let user = validate_credentials(email_address, raw_password, &mut tx).await?;

    // ユーザーがアクティブでない場合は、エラーを返却が確認
    if !user.is_active() {
        return Err(LoginError::NotActive(user.id().value()));
    }

    // セッションデータを生成
    let Settings { tokens, .. } = settings;
    let session_data = generate_session_data(user.id().value(), tokens)?;

    // セッション固定化攻撃に対する対策として、セッションを更新
    session.renew();
    // セッションデータをセッションストアに登録
    session
        .insert(&session_data)
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // ユーザーの最終ログイン日時を更新
    update_last_logged_in(user.id(), &mut tx).await?;

    // トランザクションをコミット
    tx.commit()
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // セッションデータを返却
    Ok(session_data)
}
