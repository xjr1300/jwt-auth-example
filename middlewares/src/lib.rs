//! JwtAuthMiddleware
//!
//! 保護されたリソースへのアクセスを制限するミドルウェアで、セッションID、アクセストークン及び
//! リフレッシュトークンで、トークンの`サイレントリフレッシュ`を実現する。
//!
//! リクエストヘッダーに以下のクッキーが含まれていることを想定する。
//!
//! * `session_id`: セッションID
//! * `access_token`: アクセストークン
//! * `refresh_token`: リフレッシュトークン
//!
//! このミドルウェアを経由するリクエストを受け取った場合、以下の処理をする。
//!
//! セッションIDをキーにRedisから以下を含んだ`セッションデータ`を取得する。
//!
//! * ユーザーID
//! * アクセストークン
//! * アクセストークンの有効期限(Unixエポック秒)
//! * リフレッシュトークン
//! * リフレッシュトークンの有効期限(Unixエポック秒)
//!
//! `セッションデータ`を取得できなかった場合は、即座に`401 Unauthorized`で応答するとともに、クッキーの削除
//! を応答で指示する。
//!
//! クッキーのアクセストークンと、`セッションデータ`のアクセストークンが一致するか確認して、一致しなかった場合は、
//! 即座に`401 Unauthorized`で応答するとともに、Redisに格納された当該`セッションデータ`を削除して、クッキーの
//! 削除を応答で指示する。
//!
//! 次に、`セッションデータ`のアクセストークンの有効期限を確認して、その有効期限が切れていない場合は、保護された
//! リソースへのアクセスを許可する。
//!
//! アクセストークンの有効期限が切れていた場合は、クッキーのリフレッシュトークンと`セッションデータ`のアクセス
//! トークンが一致するか確認して、一致しなかった場合は、即座に`401 Unauthorized`で応答するとともに、
//! Redisに格納された当該`セッションデータ`を削除して、クッキーの削除を応答で指示する。
//!
//! 次に、`セッションデータ`のリフレッシュトークンの有効期限を確認して、その有効期限が切れていない場合は、保護された
//! リソースへのアクセスを許可して(A)、有効期限が切切れていた場合は、即座に`401 Unauthorized`で応答するとともに、
//! Redisに格納された当該`セッションデータ`を削除して、クッキーの削除を応答で指示する。
//!
//! (A)の場合、新しいアクセストークンとリフレッシュトークンを生成して、それぞれの有効期限とともに、当該セッションID
//! をキーに`セッションデータ`として保存する。
//! また、ブラウザにセッションIDと、新しく生成したアクセストークンとリフレッシュトークンをクッキーに保存するように
//! 指示する。

use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::rc::Rc;

use actix_session::SessionExt;
use actix_web::body::MessageBody;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};

use configurations::session::TypedSession;

pub struct JwtAuth;

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = JwtAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct JwtAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, service_request: ServiceRequest) -> Self::Future {
        tracing::info!("JwtAuthMiddleware called!");

        // セッションデータを取得
        let session = TypedSession(service_request.get_session());
        let session_data = session.get();
        if let Err(e) = session_data {
            return Box::pin(ready(Err(actix_web::error::ErrorInternalServerError(e))));
        }
        tracing::info!("SessionData: {:?}", session_data);

        // 後続のミドルウェアなどにリクエストの処理を移譲
        let future = self.service.call(service_request);

        Box::pin(async move {
            // リクエストの処理が完了した後、リクエストの処理を移譲した先から返却されたフューチャーを、
            // レスポンスとして返却
            let response = future.await?;

            tracing::info!("JwtAuthMiddleware response!");
            Ok(response)
        })
    }
}
