use actix_web::{web, HttpResponse};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use domains::models::base::EmailAddress;
use usecases::auth::{self, LoginError};

use crate::responses::e400;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub email_address: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "login")]
pub async fn login(
    pool: web::Data<PgPool>,
    data: web::Json<LoginData>,
) -> Result<HttpResponse, actix_web::Error> {
    let email_address = EmailAddress::new(&data.email_address).map_err(e400)?;
    let _auth_info = auth::login(pool.as_ref(), email_address, data.password.clone())
        .await
        .map_err(|e| {
            // トレースログを出力
            tracing::error!("{}", e);
            // エラー内容に合わせてレスポンスを返却
            match e {
                LoginError::InvalidCredentials => actix_web::error::ErrorUnauthorized(e),
                LoginError::UnexpectedError(_) => actix_web::error::ErrorInternalServerError(e),
            }
        })?;

    // TODO: アクセストークンとリフレッシュトークンをクッキーに記録するように指示

    Ok(HttpResponse::Ok().finish())
}
