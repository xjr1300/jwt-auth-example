use actix_web::{web, HttpResponse};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use domains::models::base::EmailAddress;
use usecases::auth;

use crate::responses::e400;

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email_address: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "login")]
pub async fn login(
    pool: web::Data<PgPool>,
    data: web::Json<LoginBody>,
) -> Result<HttpResponse, actix_web::Error> {
    let email_address = EmailAddress::new(&data.email_address).map_err(e400)?;

    let _ = auth::login(pool.as_ref(), email_address, data.password.clone());

    Ok(HttpResponse::Ok().finish())
}