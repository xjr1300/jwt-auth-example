use actix_web::{http::header::ContentType, web, HttpResponse};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

use configurations::{
    session::{add_session_data_cookies, TypedSession},
    Settings,
};
use domains::models::{
    users::{RawPassword, UserName},
    EmailAddress,
};
use usecases::accounts::{self, LoginError, SignupError};

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
    Ok(add_session_data_cookies(
        HttpResponse::Ok(),
        &session_data,
        &settings.as_ref().session_cookie,
    )
    .finish())
}
