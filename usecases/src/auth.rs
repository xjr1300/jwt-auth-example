use secrecy::Secret;
use sqlx::PgPool;

use configurations::{telemetries::spawn_blocking_with_tracing, Settings};
use domains::models::base::EmailAddress;
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

pub async fn login(
    email_address: EmailAddress,
    raw_password: Secret<String>,
    settings: &Settings,
    pool: &PgPool,
) -> anyhow::Result<AuthInfo, LoginError> {
    // Eメールアドレスからユーザーを取得
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    let result = PgUserRepository::default()
        .by_email_address(&email_address, &mut tx)
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    if result.is_none() {
        return Err(LoginError::InvalidCredentials);
    }

    // 引数で受け取ったパスワードをハッシュ化した結果が、ユーザーに記録されているハッシュ化パスワードと一致するか確認
    let user = result.unwrap();
    let user_id = user.id().value();
    let result = spawn_blocking_with_tracing(move || {
        let expected_hashed = user.hashed_password().value();
        verify_password(expected_hashed, &raw_password)
    })
    .await
    .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    if let Err(e) = result {
        return Err(LoginError::UnexpectedError(e.into()));
    }

    // アクセストークンとリフレッシュトークンを生成
    let base_epoch = current_unix_epoch();
    let (access_token, refresh_token) = generate_jwt_pair(
        user_id,
        &settings.tokens.secret_key,
        base_epoch,
        settings.tokens.access_token_duration.as_seconds_f64() as u64,
        settings.tokens.refresh_token_duration.as_seconds_f64() as u64,
    )
    .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // TODO: アクセストークンをセッションストア（redis）に登録

    // TODO: リフレッシュトークンをデータベースに登録

    // トークンを返却
    Ok(AuthInfo {
        access_token,
        refresh_token,
    })
}
