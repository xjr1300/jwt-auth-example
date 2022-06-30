use actix_web::{web, HttpResponse};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use configurations::{session::TypedSession, Settings};
use domains::models::base::EmailAddress;
use usecases::auth::{self, LoginError};

use crate::responses::e400;

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
    let _session_data = auth::login(
        email_address,
        data.password.clone(),
        settings.as_ref(),
        &session,
        pool.as_ref(),
    )
    .await
    .map_err(|e| {
        // トレースログを出力
        tracing::error!("{}", e);
        // エラー内容に合わせてレスポンスを返却
        match e {
            LoginError::InvalidCredentials => actix_web::error::ErrorUnauthorized(e),
            LoginError::NotActive(_) => actix_web::error::ErrorUnauthorized(e),
            LoginError::UnexpectedError(_) => actix_web::error::ErrorInternalServerError(e),
        }
    })?;

    // TODO: アクセストークンとリフレッシュトークンをクッキーに記録するように指示

    Ok(HttpResponse::Ok().finish())
}
