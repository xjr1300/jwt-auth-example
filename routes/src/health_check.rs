use actix_web::HttpResponse;

/// ヘルスチェックハンドラ
///
/// # Returns
///
/// Httpレスポンス。
#[tracing::instrument(name = "health check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().body("Are you ready?")
}
