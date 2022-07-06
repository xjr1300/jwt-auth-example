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
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, HttpMessage};
use sqlx::PgPool;
use uuid::Uuid;

use configurations::{
    generate_session_data,
    session::{
        build_session_data_cookie, SessionData, TypedSession, ACCESS_TOKEN_COOKIE_NAME,
        REFRESH_TOKEN_COOKIE_NAME,
    },
    Settings,
};
use domains::models::users::{User, UserId};
use infrastructures::repositories::users::PgUserRepository;
use miscellaneous::current_unix_epoch;

pub struct JwtAuth;

impl<S> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = actix_web::Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
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

fn get_settings(service_req: &ServiceRequest) -> Result<&Settings, actix_web::Error> {
    let settings = service_req.app_data::<web::Data<Settings>>();
    if settings.is_none() {
        return Err(actix_web::error::ErrorInternalServerError(
            "システム設定を取得できませんでした。",
        ));
    }

    Ok(settings.unwrap().as_ref())
}

fn get_database_connection_pool(service_req: &ServiceRequest) -> Result<&PgPool, actix_web::Error> {
    let pool = service_req.app_data::<web::Data<PgPool>>();
    if pool.is_none() {
        return Err(actix_web::error::ErrorInternalServerError(
            "データベースコネクションプールを取得できませんでした。",
        ));
    }

    Ok(pool.unwrap().as_ref())
}

fn get_session_data(session: &TypedSession) -> Result<Option<SessionData>, actix_web::Error> {
    let session_data = session.get();
    if let Err(e) = session_data {
        return Err(actix_web::error::ErrorInternalServerError(e));
    }

    Ok(session_data.unwrap())
}

fn get_tokens(service_req: &ServiceRequest) -> (String, String) {
    let access_token = match service_req.cookie(ACCESS_TOKEN_COOKIE_NAME) {
        Some(cookie) => cookie.value().to_owned(),
        None => "".to_owned(),
    };
    let refresh_token = match service_req.cookie(REFRESH_TOKEN_COOKIE_NAME) {
        Some(cookie) => cookie.value().to_owned(),
        None => "".to_owned(),
    };

    (access_token, refresh_token)
}

#[derive(Debug, PartialEq)]
enum TokenValidation {
    /// 成功
    Succeed,
    /// リフレッシュを要求
    RequiredRefresh,
    /// 失敗
    Failure,
}

/// Redisに記録されているセッションデータと、クッキーに記録されたアクセストークンとリフレッシュトークンを評価する。
///
/// 1. リフレッシュトークンの有効期限が切れていた場合は、認証を許可できないため`失敗`を返却。
/// 2. アクセストークンの有効期限を確認して、有効期限内であればアクセストークンが一致するか確認
///   * 一致すれば`成功`を返却
///   * 一致しなければ`失敗`を返却
/// 3. アクセストークンの有効期限が切れている場合は、リフレッシュトークンが一致するか確認
///   * 一致すれば`リフレッシュ要求`を返却
///   * 一致しなければ`失敗`を返却
///
/// # Arguments
///
/// * `session_data` - Redisに記録されているセッションデータ。
/// * `access_token` - クッキーに記録されていたアクセストークン。
/// * `refresh_token` - クッキーに記録されていたリフレッシュトークン。
///
/// # Returns
///
/// * `TokenValidation::Succeed` - アクセストークンの検証に成功したため、保護されたリソースにアクセス可能。
/// * `TokenValidation::RequiredRefresh` - リフレッシュトークンの検証に成功したため、保護されたリソースにアクセス可能。
///     ただし、トークンをリフレッシュする必要がある。
/// * `TokenValidation::Failure` - トークンの検証に失敗したため、保護されたリソースにアクセス不可。
fn inspect_token_by_session_data(
    session_data: &SessionData,
    access_token: &str,
    refresh_token: &str,
) -> TokenValidation {
    // 現在日時をUnixエポック秒で取得
    let now = current_unix_epoch();

    // リフレッシュトークンの有効期限が切れている場合は`失敗`を返却
    if session_data.refresh_expiration < now {
        return TokenValidation::Failure;
    }

    // アクセストークンが有効期限ないか確認
    if now <= session_data.access_expiration {
        // アクセストークンが一致するか確認
        if session_data.access_token == access_token {
            return TokenValidation::Succeed;
        } else {
            return TokenValidation::Failure;
        }
    }

    // リフレッシュトークンが一致するか確認
    if session_data.refresh_token == refresh_token {
        TokenValidation::RequiredRefresh
    } else {
        TokenValidation::Failure
    }
}

async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<User, actix_web::Error> {
    let user_id = UserId::new(user_id);
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{}", e)))?;
    let user = PgUserRepository::default()
        .get_by_id(user_id, &mut tx)
        .await
        .map_err(|e| actix_web::error::ErrorUnauthorized(format!("{}", e)))?;
    if user.is_none() {
        return Err(actix_web::error::ErrorUnauthorized(
            "セッションデータに含まれているユーザーは存在しません。",
        ));
    }

    Ok(user.unwrap())
}

impl<S> Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = actix_web::Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, service_req: ServiceRequest) -> Self::Future {
        tracing::info!("JwtAuthMiddlewareが要求を受け取りました。");

        let service = Rc::clone(&self.service);

        #[allow(clippy::redundant_closure)]
        Box::pin(async move {
            // システム設定を取得
            let settings = get_settings(&service_req)?;
            let Settings {
                tokens,
                session_cookie,
                ..
            } = settings;
            let session_cookie = session_cookie.to_owned();
            tracing::info!("システム設定: {:?}", settings);
            // データベースコネクションプールを取得
            let pool = get_database_connection_pool(&service_req)?;
            tracing::info!("データベースコネクションプール: {:?}", pool);
            // セッションデータを取得
            let session = TypedSession(service_req.get_session());
            let session_data = get_session_data(&session)?;
            // セッションデータがない場合は、`401 Unauthorized`で応答
            if session_data.is_none() {
                return Err(actix_web::error::ErrorUnauthorized("認証されていません。"));
            }
            let mut session_data = session_data.unwrap();
            tracing::info!("セッションデータ: {:?}", session_data);
            // トークンを取得
            let (access_token, refresh_token) = get_tokens(&service_req);
            // Redisに格納されているセッションデータと、クッキーに記録されていたトークンを評価
            let result =
                inspect_token_by_session_data(&session_data, &access_token, &refresh_token);
            if result == TokenValidation::Failure {
                return Err(actix_web::error::ErrorUnauthorized("認証されていません。"));
            }
            // トークンを更新する必要がある場合は、トークンを更新したセッションデータを作成
            if result == TokenValidation::RequiredRefresh {
                session_data = generate_session_data(session_data.user_id, tokens)
                    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            }

            // リクエストにユーザーをデータとして追加
            let user = get_user(pool, session_data.user_id).await?;
            service_req.extensions_mut().insert(user);

            // 後続のミドルウェアなどにリクエストの処理を移譲
            let future = service.call(service_req);

            // リクエストの処理が完了した後、リクエストの処理を移譲した先から返却されたフューチャーを、
            // レスポンスとして返却
            let mut resp = future.await?;

            // トークンを更新する必要がある場合は、トークンを更新してRedisに記録するとともに、
            // ブラウザにトークンをクッキーに記録するように指示
            if result == TokenValidation::RequiredRefresh {
                let response = resp.response_mut();
                // Redisにセッションデータを登録
                session
                    .insert(&session_data)
                    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
                // ブラウザにトークンをクッキーに記録するように指示
                let access_token_cookie = build_session_data_cookie(
                    ACCESS_TOKEN_COOKIE_NAME,
                    &session_data.access_token,
                    &session_cookie,
                );
                let refresh_token_cookie = build_session_data_cookie(
                    REFRESH_TOKEN_COOKIE_NAME,
                    &session_data.refresh_token,
                    &session_cookie,
                );
                response.add_cookie(&access_token_cookie).unwrap();
                response.add_cookie(&refresh_token_cookie).unwrap();
            }

            tracing::info!("JwtAuthMiddlewareが応答を返しました。");
            Ok(resp)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_token_by_session_data_succeed() {
        let now = current_unix_epoch();
        let access_token = "foo";
        let refresh_token = "bar";
        let session_data = SessionData {
            user_id: Uuid::new_v4(),
            access_token: access_token.to_owned(),
            access_expiration: now + 300,
            refresh_token: refresh_token.to_owned(),
            refresh_expiration: now + 1800,
        };
        let result = inspect_token_by_session_data(&session_data, access_token, refresh_token);
        assert_eq!(result, TokenValidation::Succeed);
    }

    #[test]
    fn inspect_token_by_session_data_required_refresh() {
        let now = current_unix_epoch();
        let access_token = "foo";
        let refresh_token = "bar";

        let session_data = SessionData {
            user_id: Uuid::new_v4(),
            access_token: "baz".to_owned(),
            access_expiration: now - 1,
            refresh_token: refresh_token.to_owned(),
            refresh_expiration: now + 1800,
        };
        let result = inspect_token_by_session_data(&session_data, access_token, refresh_token);
        assert_eq!(result, TokenValidation::RequiredRefresh);
    }

    #[test]
    fn inspect_token_by_session_data_failure_for_refresh_token_expiration() {
        let now = current_unix_epoch();
        let access_token = "foo";
        let refresh_token = "bar";
        let session_data = SessionData {
            user_id: Uuid::new_v4(),
            access_token: access_token.to_owned(),
            access_expiration: now + 300,
            refresh_token: refresh_token.to_owned(),
            refresh_expiration: now - 1,
        };
        let result = inspect_token_by_session_data(&session_data, access_token, refresh_token);
        assert_eq!(result, TokenValidation::Failure);
    }

    #[test]
    fn inspect_token_by_session_data_failure_for_access_token() {
        let now = current_unix_epoch();
        let access_token = "foo";
        let refresh_token = "bar";
        let session_data = SessionData {
            user_id: Uuid::new_v4(),
            access_token: "baz".to_owned(),
            access_expiration: now + 300,
            refresh_token: refresh_token.to_owned(),
            refresh_expiration: now + 1800,
        };
        let result = inspect_token_by_session_data(&session_data, access_token, refresh_token);
        assert_eq!(result, TokenValidation::Failure);
    }

    #[test]
    fn inspect_token_by_session_data_failure_for_refresh_token() {
        let now = current_unix_epoch();
        let access_token = "foo";
        let refresh_token = "bar";
        let session_data = SessionData {
            user_id: Uuid::new_v4(),
            access_token: access_token.to_owned(),
            access_expiration: now - 1,
            refresh_token: "baz".to_owned(),
            refresh_expiration: now + 1800,
        };
        let result = inspect_token_by_session_data(&session_data, access_token, refresh_token);
        assert_eq!(result, TokenValidation::Failure);
    }
}
