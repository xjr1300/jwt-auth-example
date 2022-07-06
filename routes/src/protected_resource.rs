use actix_web::{web, HttpResponse};

use domains::models::users::User;

/// サンプル保護リソースハンドラ
///
/// # Returns
///
/// Httpレスポンス。
#[tracing::instrument(name = "Sample protected resource")]
pub async fn protected_resource(user: web::ReqData<User>) -> HttpResponse {
    HttpResponse::Ok().body(user.id().value().to_string())
}
