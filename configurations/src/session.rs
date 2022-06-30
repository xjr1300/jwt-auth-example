use std::future::{ready, Ready};

use actix_session::{Session, SessionExt};
use actix_web::{dev::Payload, FromRequest, HttpRequest};

/// 型付けセッション構造体
///
/// RedisにセッションIDをキーにアクセストークンを記録する。
pub struct TypedSession(Session);

impl TypedSession {
    /// Redisに登録するユーザーIDキー名
    const ACCESS_TOKEN_KEY: &'static str = "access_token";

    /// セッションを更新する。
    pub fn renew(&self) {
        self.0.renew();
    }

    /// アクセストークンを登録する。
    ///
    /// # Arguments
    ///
    /// * `token` - アクセストークン。
    pub fn insert(&self, token: &str) -> Result<(), serde_json::Error> {
        self.0.insert(Self::ACCESS_TOKEN_KEY, token)
    }

    /// セッションに紐づけられたアクセストークンを取得する。
    ///
    /// # Returns
    ///
    /// アクセストークン。
    pub fn token(&self) -> Result<Option<String>, serde_json::Error> {
        self.0.get(Self::ACCESS_TOKEN_KEY)
    }

    /// セッションに紐づけられたアクセストークンを削除する。
    pub fn delete(&self) {
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
