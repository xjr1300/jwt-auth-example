use std::future::{ready, Ready};

use actix_web::dev::{forward_ready, Service, Transform};

pub struct JwtAuth;

impl<S: Service<Req>, Req> Transform<S, Req> for JwtAuth {
    type Response = S::Response;
    type Error = S::Error;
    type Transform = JwtAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware { service }))
    }
}

/// JwtAuthMiddleware
///
/// 保護されたリソースへのアクセスを制限するミドルウェアで、セッションID、アクセストークン及び
/// リフレッシュトークンで、トークンの`サイレントリフレッシュ`を実現する。
///
/// リクエストヘッダーに以下のクッキーが含まれていることを想定する。
///
/// * `session_id`: セッションID
/// * `access_token`: アクセストークン
/// * `refresh_token`: リフレッシュトークン
///
/// このミドルウェアを経由するリクエストを受け取った場合、以下の処理をする。
///
/// セッションIDをキーにRedisから以下を含んだ`セッションデータ`を取得する。
///
/// * ユーザーID
/// * アクセストークン
/// * アクセストークンの有効期限(Unixエポック秒)
/// * リフレッシュトークン
/// * リフレッシュトークンの有効期限(Unixエポック秒)
///
/// `セッションデータ`を取得できなかった場合は、即座に`401 Unauthorized`で応答するとともに、クッキーの削除
/// を応答で指示する。
///
/// クッキーのアクセストークンと、`セッションデータ`のアクセストークンが一致するか確認して、一致しなかった場合は、
/// 即座に`401 Unauthorized`で応答するとともに、Redisに格納された当該`セッションデータ`を削除して、クッキーの
/// 削除を応答で指示する。
///
/// 次に、`セッションデータ`のアクセストークンの有効期限を確認して、その有効期限が切れていない場合は、保護された
/// リソースへのアクセスを許可する。
///
/// アクセストークンの有効期限が切れていた場合は、クッキーのリフレッシュトークンと`セッションデータ`のアクセス
/// トークンが一致するか確認して、一致しなかった場合は、即座に`401 Unauthorized`で応答するとともに、
/// Redisに格納された当該`セッションデータ`を削除して、クッキーの削除を応答で指示する。
///
/// 次に、`セッションデータ`のリフレッシュトークンの有効期限を確認して、その有効期限が切れていない場合は、保護された
/// リソースへのアクセスを許可して(A)、有効期限が切切れていた場合は、即座に`401 Unauthorized`で応答するとともに、
/// Redisに格納された当該`セッションデータ`を削除して、クッキーの削除を応答で指示する。
///
/// (A)の場合、新しいアクセストークンとリフレッシュトークンを生成して、それぞれの有効期限とともに、当該セッションID
/// をキーに`セッションデータ`として保存する。
/// また、ブラウザにセッションIDと、新しく生成したアクセストークンとリフレッシュトークンをクッキーに保存するように
/// 指示する。
pub struct JwtAuthMiddleware<S> {
    service: S,
}

impl<S: Service<Req>, Req> Service<Req> for JwtAuthMiddleware<S> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    forward_ready!(service);

    fn call(&self, req: Req) -> Self::Future {
        println!("JwtAuthMiddleware called!");
        self.service.call(req)
    }
}
