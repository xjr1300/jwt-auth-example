use actix_web::{web, HttpResponse};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use usecases::auth;

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email_address: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "login")]
pub async fn login(pool: web::Data<PgPool>, data: web::Json<LoginBody>) -> HttpResponse {
    let _ = auth::login(pool.as_ref(), &data.email_address, &data.password);

    HttpResponse::Ok().finish()
}
