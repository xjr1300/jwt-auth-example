use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{fmt::writer::MakeWriterExt, EnvFilter};

use web_server::configurations::{DatabaseSettings, ENV_VALUES, WebAppSettings};
use web_server::telemetries::{get_subscriber, init_subscriber};

#[tracing::instrument(name = "Hello world")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // トレーシングログを設定
    let path = std::env::current_dir().expect("カレントディレクトリの検知に失敗しました。");
    let log_dir = path.join("logs");
    let log_file = tracing_appender::rolling::daily(log_dir, "web");
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(ENV_VALUES.rust_log.clone()));
    let subscriber = get_subscriber(
        "jwt-auth-example".into(),
        env_filter,
        std::io::stdout.and(log_file), // 標準出力とファイルにログを出力
    );
    init_subscriber(subscriber);

    // データベースに接続
    let database_settings = DatabaseSettings::default();
    tracing::info!("Connect to database...");
    let pool = web::Data::new(PgPoolOptions::new().connect_lazy_with(database_settings.with_db()));

    // アプリケーションを起動
    tracing::info!("Startup server...");
    let web_app_settings = WebAppSettings::default();
    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .route("/", web::get().to(hello))
    })
    .bind(web_app_settings.socket_address())?
    .run()
    .await?;

    Ok(())
}
