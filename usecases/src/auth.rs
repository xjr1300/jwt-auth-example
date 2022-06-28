use secrecy::Secret;
use sqlx::PgPool;

pub struct Tokens {
    pub access: String,
    pub refresh: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("アカウントが見つかりません。")]
    AccountNotFound,
    #[error("Eメールアドレスまたはパスワードが異なります。")]
    InvalidCredentials,
    #[error("内部サーバーエラーが発生しました。")]
    InternalError,
}

pub fn login(
    _pool: &PgPool,
    _email_address: &str,
    _raw_password: &Secret<String>,
) -> Result<Tokens, LoginError> {
    Ok(Tokens {
        access: "access".to_owned(),
        refresh: "refresh".to_owned(),
    })
}
