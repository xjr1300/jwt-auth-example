use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::anyhow;
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use secrecy::{ExposeSecret, Secret};
use sha2::Sha256;
use uuid::Uuid;

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
