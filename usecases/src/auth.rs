use secrecy::Secret;
use sqlx::{PgPool, Postgres, Transaction};

use configurations::{session::TypedSession, telemetries::spawn_blocking_with_tracing, Settings};
use domains::models::{base::EmailAddress, users::User};
use hashed_password::{current_unix_epoch, generate_jwt_pair, verify_password};
use infrastructures::repositories::users::PgUserRepository;

pub struct AuthInfo {
    /// アクセストークン
    pub access_token: String,
    /// リフレッシュトークン
    pub refresh_token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Eメールアドレスまたはパスワードが異なります。")]
    InvalidCredentials,
    #[error("想定していないエラーが発生しました。{0}")]
    UnexpectedError(#[from] anyhow::Error),
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
        .by_email_address(&email_address, tx)
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
        return Err(LoginError::UnexpectedError(e.into()));
    }

    Ok(user)
}

pub async fn login(
    email_address: EmailAddress,
    raw_password: Secret<String>,
    settings: &Settings,
    session: &TypedSession,
    pool: &PgPool,
) -> anyhow::Result<AuthInfo, LoginError> {
    // トランザクションを開始
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // データベースからユーザーを取得して、パスワードを検証
    let user = validate_credentials(email_address, raw_password, &mut tx).await?;

    // アクセストークンとリフレッシュトークンを生成
    let Settings { tokens, .. } = settings;
    let base_epoch = current_unix_epoch();
    let (access_token, refresh_token) = generate_jwt_pair(
        user.id().value(),
        &settings.tokens.secret_key,
        base_epoch,
        tokens.access_token_duration(),
        tokens.refresh_token_duration(),
    )
    .map_err(LoginError::UnexpectedError)?;

    // セッションを更新して、アクセストークンをセッションストア（redis）に登録
    session.renew();
    session
        .insert(&access_token)
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // TODO: リフレッシュトークンをデータベースに登録

    // トークンを返却
    Ok(AuthInfo {
        access_token,
        refresh_token,
    })
}
