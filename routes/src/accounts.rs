use actix_web::{http::header::ContentType, web, HttpResponse};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    email_address: String,
    password: Secret<String>,
}

#[tracing::instrument(name = "login")]
pub async fn login(body: web::Json<LoginBody>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!(
            "Welcome {}! Secret is {}",
            body.email_address,
            body.password.expose_secret(),
        ))
}
