use std::future::{ready, Ready};

use actix_session::{Session, SessionExt};
use actix_web::{cookie::Cookie, dev::Payload, FromRequest, HttpRequest};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::SessionCookieSettings;

pub const ACCESS_TOKEN_COOKIE_NAME: &str = "access_token";
pub const REFRESH_TOKEN_COOKIE_NAME: &str = "refresh_token";

/// セッションデータ構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// ユーザーID
    pub user_id: Uuid,
    /// アクセストークン
    pub access_token: String,
    /// アクセストークン有効期限（UNIXエポック秒）
    pub access_expiration: u64,
    /// リフレッシュトークン
    pub refresh_token: String,
    /// リフレッシュトークン有効期限（UNIXエポック秒）
    pub refresh_expiration: u64,
}

/// 型付けセッション構造体
///
/// RedisにセッションIDをキーにアクセストークンを記録する。
pub struct TypedSession(pub Session);

impl TypedSession {
    const SESSION_DATA_KEY: &'static str = "session_data";

    /// セッションデータを取得する。
    ///
    /// # Returns
    ///
    /// セッションデータ。
    pub fn get(&self) -> Result<Option<SessionData>, serde_json::Error> {
        self.0.get(Self::SESSION_DATA_KEY)
    }

    /// セッションデータを登録する。
    ///
    /// # Arguments
    ///
    /// * `data` - セッションデータ。
    pub fn insert(&self, data: &SessionData) -> Result<(), serde_json::Error> {
        self.0.insert(Self::SESSION_DATA_KEY, data)
    }

    /// セッションデータを削除する。
    pub fn remove(&self) -> Option<String> {
        self.0.remove(Self::SESSION_DATA_KEY)
    }

    /// セッションをクリアする。
    pub fn clear(&self) {
        self.0.clear()
    }

    /// セッションを更新する。
    ///
    /// 既存のセッションデータは、新しいセッションIDに割り当てられる。
    pub fn renew(&self) {
        self.0.renew();
    }

    /// セッションストアからセッションデータを削除して、クライアントのセッションを削除する。
    pub fn purge(&self) {
        self.0.purge()
    }
}

impl FromRequest for TypedSession {
    // Sessionが実装するFromRequestによって返却される同じ型のエラーをエラーとして定義
    type Error = <Session as FromRequest>::Error;

    // Rustはトレイトにおける`async`構文をサポートしていない。
    // FromRequestは、HTTP呼び出しなど非同期で操作するために、戻り値の型としてFutureを想定している。
    // しかし、TypedSession構造体には、I/O動作などの非同期操作がないため、Futureを持たない。
    // このため、TypedSessionをReadyでラップして、エグゼキューターによって最初にポーリングされたときに、
    // ラップされた値に解決するFutureに変換するために、TypedSessionをReadyでラップする。
    type Future = Ready<Result<TypedSession, Self::Error>>;

    /// リクエストから型付けセッションを取得する。
    ///
    /// # Arguments
    ///
    /// * `request` - HTTPリクエスト。
    /// * `_payload` - ペイロード。
    ///
    /// # Returns
    ///
    /// 型付けセッション。
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}

pub fn build_session_data_cookie<'a>(
    name: &'a str,
    value: &'a str,
    settings: &'a SessionCookieSettings,
) -> Cookie<'a> {
    Cookie::build(name.to_owned(), value.to_owned())
        .path("/")
        .secure(settings.secure.to_owned())
        .http_only(true)
        .same_site(settings.same_site.to_owned())
        .finish()
        .into_owned()
}
