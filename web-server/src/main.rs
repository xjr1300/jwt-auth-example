use dotenvy::dotenv;
use tracing_subscriber::{fmt::writer::MakeWriterExt, EnvFilter};

use configurations::telemetries::{get_subscriber, init_subscriber};
use configurations::Settings;

use web_server::startup::WebApp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // 設定を取得
    let settings = Settings::default();

    // トレーシングログを設定
    let path = std::env::current_dir().expect("カレントディレクトリの検知に失敗しました。");
    let log_dir = path.join("logs");
    let log_file = tracing_appender::rolling::daily(log_dir, "web");
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(settings.rust_log.clone()));
    let subscriber = get_subscriber(
        "jwt-auth-example".into(),
        env_filter,
        std::io::stdout.and(log_file), // 標準出力とファイルにログを出力
    );
    init_subscriber(subscriber);

    // Webアプリを起動
    let web_app = WebApp::build(settings.clone()).await?;
    web_app.run_until_stopped().await?;

    Ok(())
}
