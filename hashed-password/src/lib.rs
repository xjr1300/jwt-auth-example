use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
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
///
/// ```
/// /* cSpell: disable */
/// $argon2id$v=19$m=65536,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno
/// /* cSpell: enable */
/// ```
pub fn hashed_password(password: &Secret<String>) -> Result<Secret<String>, anyhow::Error> {
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
