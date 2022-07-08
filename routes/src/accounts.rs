use actix_web::{cookie::Cookie, http::header::ContentType, web, HttpResponse};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

use configurations::{
    session::{
        add_session_data_cookies, TypedSession, ACCESS_TOKEN_COOKIE_NAME, REFRESH_TOKEN_COOKIE_NAME,
    },
    Settings,
};
use domains::models::{
    users::{RawPassword, User, UserName},
    EmailAddress,
};
use middlewares::JwtAuth;
use usecases::accounts::{self, ChangePasswordError, LoginError, SignupError};

use crate::responses::e400;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupData {
    pub user_name: String,
    pub email_address: String,
    pub password: Secret<String>,
}

#[tracing::instrument(skip(pool), name = "Signup")]
pub async fn signup(
    data: web::Json<SignupData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_name = UserName::new(&data.user_name).map_err(e400)?;
    let email_address = EmailAddress::new(&data.email_address).map_err(e400)?;
    let password = RawPassword::new(data.password.expose_secret()).map_err(e400)?;
    let user = accounts::signup(user_name, email_address, password, &pool)
        .await
        .map_err(|e| {
            tracing::error!("{:?}", e);
            match e {
                SignupError::EmailAddressAlreadyExists => actix_web::error::ErrorBadRequest(e),
                SignupError::UnexpectedError(_) => actix_web::error::ErrorInternalServerError(e),
            }
        })?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(user))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub email_address: String,
    pub password: Secret<String>,
}

#[tracing::instrument(skip(session, pool), name = "Login user")]
pub async fn login(
    data: web::Json<LoginData>,
    settings: web::Data<Settings>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let email_address = EmailAddress::new(&data.email_address).map_err(e400)?;
    let session_data = accounts::login(
        email_address,
        data.password.clone(),
        settings.as_ref(),
        &session,
        &pool,
    )
    .await
    .map_err(|e| {
        tracing::error!("{:?}", e);
        match e {
            LoginError::UnexpectedError(_) => actix_web::error::ErrorInternalServerError(e),
            LoginError::InvalidCredentials => actix_web::error::ErrorUnauthorized(e),
            LoginError::NotActive(_) => actix_web::error::ErrorUnauthorized(e),
        }
    })?;

    // セッションデータをクッキーに追加するように指示してレスポンスを返却
    let mut response = HttpResponse::Ok().finish();
    add_session_data_cookies(
        &mut response,
        &session_data.access_token,
        &session_data.refresh_token,
        &settings.session_cookie,
    );

    Ok(response)
}

/// 有効期限の切れたトークンを記録するクッキーを作成する。
fn create_expired_token_cookies<'a>() -> (Cookie<'a>, Cookie<'a>) {
    let mut access = Cookie::new(ACCESS_TOKEN_COOKIE_NAME, "");
    access.make_removal();
    let mut refresh = Cookie::new(REFRESH_TOKEN_COOKIE_NAME, "");
    refresh.make_removal();

    (access, refresh)
}

#[tracing::instrument(skip(session), name = "Logout user")]
pub async fn logout(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    // クッキーに記録しているセッションIDを削除するようにブラウザに指示して、Redisからセッションデータを削除
    session.purge();
    // 有効期限のないトークン用のクッキーを生成
    let (access_token_cookie, refresh_token_cookie) = create_expired_token_cookies();

    // パスワード変更に成功したら、ブラウザにクッキーを削除するように指示
    Ok(HttpResponse::Ok()
        .cookie(access_token_cookie)
        .cookie(refresh_token_cookie)
        .finish())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordData {
    pub old_password: Secret<String>,
    pub new_password: Secret<String>,
}

#[tracing::instrument(skip(session, pool), name = "Change password")]
pub async fn change_password(
    user: web::ReqData<User>,
    data: web::Json<ChangePasswordData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let old_password = RawPassword::new(data.old_password.expose_secret()).map_err(e400)?;
    let new_password = RawPassword::new(data.new_password.expose_secret()).map_err(e400)?;

    accounts::change_password(&user, old_password, new_password, &session, pool.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("{:?}", e);
            match e {
                ChangePasswordError::UnexpectedError(_) => {
                    actix_web::error::ErrorInternalServerError(e)
                }
                ChangePasswordError::IncorrectCurrentPassword => {
                    actix_web::error::ErrorBadRequest(e)
                }
                ChangePasswordError::NotFound(_) => actix_web::error::ErrorBadRequest(e),
            }
        })?;

    // 有効期限のないトークン用のクッキーを生成
    let (access_token_cookie, refresh_token_cookie) = create_expired_token_cookies();

    // パスワード変更に成功したら、ブラウザにクッキーを削除するように指示
    Ok(HttpResponse::Ok()
        .cookie(access_token_cookie)
        .cookie(refresh_token_cookie)
        .finish())
}

/// アカウントスコープを返却する。
pub fn accounts_scope() -> actix_web::Scope {
    web::scope("/accounts")
        .service(web::resource("/signup").route(web::post().to(signup)))
        .service(web::resource("/login").route(web::post().to(login)))
        .service(
            web::scope("")
                .wrap(JwtAuth)
                .service(web::resource("/logout").route(web::post().to(logout)))
                .service(web::resource("/change_password").route(web::post().to(change_password))),
        )
}
