use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use secrecy::{ExposeSecret, Secret};

/// パスワードをハッシュ化した文字列をPHCフォーマットで返却する。
///
/// パスワードに生成したソルトを付与して、ハッシュ化する。
///
/// # Arguments
///
/// * `password`: パスワードインスタンス。
///
/// # Returns
///
/// ソルトを付与したハッシュ化したパスワードのPHC文字列。
pub fn compute_hashed_password(password: &Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15_000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(Secret::new(password_hash))
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("ユーザークレデンシャルが不正です。")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// パスワードを検証する。
///
/// # Arguments
///
/// * `expected_hashed` - データベースに保存されているハッシュ化したユーザーのパスワード。
/// * `raw_password` - ユーザー認証する際に、ユーザーがパスワードとして入力した文字列。
///
/// # Returns
///
/// パスワードの検証に成功した場合は`()`。
pub fn verify_password(
    expected_hashed: &Secret<String>,
    raw_password: &Secret<String>,
) -> Result<(), AuthError> {
    // PHC文字列をパースしてパスワードハッシュを取得
    let expected_hashed = PasswordHash::new(expected_hashed.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    // 提供されたパスワードハッシュのパラメーターを使用して、提供されたパスワードに対してこのパスワードハッシュ関数を
    // 計算して、計算された結果が一致するか確認
    Argon2::default()
        .verify_password(raw_password.expose_secret().as_bytes(), &expected_hashed)
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}
