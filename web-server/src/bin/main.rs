use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use dotenvy::dotenv;
use tracing_subscriber::{fmt::writer::MakeWriterExt, EnvFilter};

use web_server::telemetries::{get_subscriber, init_subscriber};

#[tracing::instrument(name = "Hello world")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let path = std::env::current_dir().expect("カレントディレクトリの検知に失敗しました。");
    let log_dir = path.join("logs");
    let log_file = tracing_appender::rolling::daily(log_dir, "web");
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = get_subscriber(
        "jwt-auth-example".into(),
        env_filter,
        std::io::stdout.and(log_file), // 標準出力とファイルにログを出力
    );
    init_subscriber(subscriber);

    tracing::info!("Startup server...");
    HttpServer::new(move || App::new().route("/", web::get().to(hello)))
        .bind("127.0.0.1:8000")?
        .run()
        .await?;

    Ok(())
}
