use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use secrecy::{ExposeSecret, Secret};
use sha2::Sha256;
use uuid::Uuid;

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
pub fn compute_hashed_password(password: &Secret<String>) -> anyhow::Result<Secret<String>> {
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

/// 有効期限の開始を指定したJWTを生成する。
///
/// # Arguments
///
/// * `user_id` - ユーザーID。
/// * `secret` - JWT生成鍵。
/// * `expiration` - トークンの有効期限を示すUNIXエポック秒。
///
/// # Returns
///
/// JWT。
fn generate_jwt(
    user_id: Uuid,
    secret_key: &Secret<String>,
    expiration: u64,
) -> anyhow::Result<String> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.expose_secret().as_bytes())?;
    let mut claims = BTreeMap::new();
    claims.insert("sub", user_id.to_string());
    claims.insert("exp", expiration.to_string());

    Ok(claims.sign_with_key(&key)?)
}

/// アクセストークンとリフレッシュトークンを生成する。
///
/// # Arguments
///
/// * `user_id` - ユーザーID。
/// * `secret` - JWT生成鍵。
/// * `access_expiration` - アクセストークンの有効期限を示すUNIXエポック秒。
/// * `refresh_expiration` - リフレッシュトークンの有効期限を示すUNIXエポック秒。
///
/// # Returns
///
/// アクセストークンとリフレッシュトークンを格納したタプル
pub fn generate_jwt_pair(
    user_id: Uuid,
    secret_key: &Secret<String>,
    access_expiration: u64,
    refresh_expiration: u64,
) -> anyhow::Result<(String, String)> {
    Ok((
        generate_jwt(user_id, secret_key, access_expiration)?,
        generate_jwt(user_id, secret_key, refresh_expiration)?,
    ))
}

/// クレーム構造体
pub struct Claim {
    /// ユーザーID。
    pub user_id: Uuid,
    /// 有効期限を示すUNIXエポック秒。
    pub expiration: u64,
}

/// JWTからクレームを取得する。
///
/// * `token` - JWT。
/// * `secret` - JWT生成鍵。
///
/// # Returns
///
/// クレーム。
pub fn get_claim_from_jwt(token: &str, secret_key: &Secret<String>) -> anyhow::Result<Claim> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.expose_secret().as_bytes())?;
    let claims: BTreeMap<String, String> = token.verify_with_key(&key)?;
    // ユーザーIDを取得
    let user_id = Uuid::from_str(
        claims
            .get("sub")
            .ok_or_else(|| anyhow!("JWTにsubが含まれていません。"))?,
    )
    .map_err(|_| anyhow!("JWTに含まれているユーザーIDが不正です。"))?;
    // 有効期限を取得
    let expiration: u64 = claims
        .get("exp")
        .ok_or_else(|| anyhow!("JWTにexpが含まれていません。"))?
        .parse()
        .map_err(|_| anyhow!("JWTに含まれている有効期限が不正です。"))?;

    Ok(Claim {
        user_id,
        expiration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use miscellaneous::current_unix_epoch;
    use uuid::Uuid;

    /// パスワードを正常にハッシュ化できることを確認するテスト
    #[test]
    fn test_hashed_password() {
        let password = Secret::new("some-password".to_owned());
        let hashed = compute_hashed_password(&password).unwrap();
        assert!(verify_password(&hashed, &password).is_ok())
    }

    /// JWTを正常に生成できることを確認するテスト
    #[test]
    fn test_generate_jwt() {
        // JWTを生成
        let user_id = Uuid::new_v4();
        let secret_key = Secret::new("some-secret".to_owned());
        let now = current_unix_epoch();
        let duration: u64 = 300;
        let token = generate_jwt(user_id, &secret_key, now + duration).unwrap();
        // JWTを検証
        let claim = get_claim_from_jwt(&token, &secret_key).unwrap();
        assert_eq!(claim.user_id, user_id);
        assert_eq!(claim.expiration, now + duration);
    }

    /// 異なるアクセストークンとリフレッシュトークンを作成することを確認するテスト
    #[test]
    fn test_generate_jwt_pair() {
        let user_id = Uuid::new_v4();
        let secret_key = Secret::new("some-secret".to_owned());
        let now = current_unix_epoch();
        let access_expiration: u64 = now + 300;
        let refresh_expiration: u64 = now + 3600;
        let (access, refresh) =
            generate_jwt_pair(user_id, &secret_key, access_expiration, refresh_expiration).unwrap();
        assert_ne!(
            access, refresh,
            "アクセストークンとリフレッシュトークンが同じです。"
        )
    }
}
