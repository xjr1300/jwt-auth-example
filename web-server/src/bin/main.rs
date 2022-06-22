use actix_web::{web, App, HttpResponse, HttpServer, Responder};

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    HttpServer::new(move || App::new().route("/", web::get().to(hello)))
        .bind("127.0.0.1:8000")?
        .run()
        .await?;

    return Ok(());
}
