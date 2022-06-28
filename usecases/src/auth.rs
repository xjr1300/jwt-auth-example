use secrecy::Secret;
use sqlx::PgPool;

use domains::models::base::EmailAddress;
use hashed_password::verify_password;
use infrastructures::repositories::users::PgUserRepository;
use telemetries::spawn_blocking_with_tracing;

pub struct Tokens {
    pub access: String,
    pub refresh: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Eメールアドレスまたはパスワードが異なります。")]
    InvalidCredentials,
    #[error("想定していないエラーが発生しました。{0}")]
    UnexpectedError(#[from] anyhow::Error),
}

pub async fn login(
    pool: &PgPool,
    email_address: EmailAddress,
    raw_password: Secret<String>,
) -> anyhow::Result<Tokens, LoginError> {
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
    let result = spawn_blocking_with_tracing(move || {
        let expected_hashed = user.hashed_password().value();
        verify_password(expected_hashed, &raw_password)
    })
    .await
    .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    if let Err(e) = result {
        return Err(LoginError::UnexpectedError(e.into()));
    }

    Ok(Tokens {
        access: "access".to_owned(),
        refresh: "refresh".to_owned(),
    })
}
