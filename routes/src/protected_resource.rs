use actix_web::HttpResponse;

/// サンプル保護リソースハンドラ
///
/// # Returns
///
/// Httpレスポンス。
#[tracing::instrument(name = "Sample protected resource")]
pub async fn sample_protected_resource() -> HttpResponse {
    HttpResponse::Ok().body("This is a protected resource!")
}
