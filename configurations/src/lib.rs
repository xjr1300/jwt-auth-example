mod settings;

pub use settings::*;
pub mod password;
pub mod session;
pub mod telemetries;
pub mod tokens;

use anyhow::anyhow;
use miscellaneous::current_unix_epoch;
use session::SessionData;
use tokens::generate_jwt_pair;
use uuid::Uuid;

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
pub fn generate_session_data(
    user_id: Uuid,
    token_settings: &TokensSettings,
) -> Result<SessionData, anyhow::Error> {
    let base_epoch = current_unix_epoch();
    let access_expiration = base_epoch + token_settings.access_token_duration();
    let refresh_expiration = base_epoch + token_settings.refresh_token_duration();
    let (access_token, refresh_token) = generate_jwt_pair(
        user_id,
        &token_settings.secret_key,
        access_expiration,
        refresh_expiration,
    )
    .map_err(|e| {
        anyhow!(format!(
            "JWTトークンペアを生成するときにエラーが発生しました。{}",
            e
        ))
    })?;

    Ok(SessionData {
        user_id,
        access_token,
        access_expiration,
        refresh_token,
        refresh_expiration,
    })
}
