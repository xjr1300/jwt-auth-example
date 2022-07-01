use dotenvy::dotenv;
use once_cell::sync::Lazy;
use sqlx::{postgres::PgPool, Connection, Executor, PgConnection};
use uuid::Uuid;

use configurations::telemetries::{get_subscriber, init_subscriber};
use configurations::{DatabaseSettings, Settings};
use web_server::startup::{get_connection_pool, WebApp};

use crate::users::TestUsers;

/// 環境変数にTEST_LOGがあった場合、トレースを標準出力に出力して、std::io::Sinkに出力する。
/// std::io::sinkは、すべてのデータを消費するライターインスタンスを構築する関数である。
static TRACING: Lazy<()> = Lazy::new(|| {
    let name = "test".to_string();
    let level = "info".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(name, level.into(), std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(name, level.into(), std::io::sink);
        init_subscriber(subscriber);
    };
});

/// テスト用Webアプリ構造体
pub struct TestWebApp {
    pub web_app_address: String,
    pub port: u16,
    pub pool: PgPool,
    pub api_client: reqwest::Client,
    pub test_users: TestUsers,
}

/// テスト用Webアプリを生成する。
pub async fn spawn_web_app() -> TestWebApp {
    dotenv().ok();

    Lazy::force(&TRACING);

    let settings = {
        let mut s = Settings::default();
        s.web_app.port = 0; // OSにポート番号を指定してもらうようにポート0を設定
        s.db.database_name = Uuid::new_v4().to_string(); // 新しいテスト用のデータベース

        s
    };

    // テスト用のデータベースを作成してマイグレート
    configure_database(&settings.db).await;

    let web_app = WebApp::build(settings.clone())
        .await
        .expect("テスト用Webあアプリの構築に失敗しました。");
    let port = web_app.port();
    let _ = tokio::spawn(web_app.run_until_stopped());

    // APIクライアントを構築
    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let web_app = TestWebApp {
        web_app_address: format!("http://localhost:{}", port),
        port,
        pool: get_connection_pool(&settings.db),
        api_client,
        test_users: TestUsers::default(),
    };

    web_app
}

async fn configure_database(settings: &DatabaseSettings) -> PgPool {
    // データベース名を指定しないことで、template1データベースに接続
    let mut connection = PgConnection::connect_with(&settings.without_db())
        .await
        .expect("Fail to connect to postgres.");
    // テスト用データベースを構築
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, settings.database_name).as_str())
        .await
        .expect("Failed to create test database.");

    // テスト用データベースに接続して、マイグレーションを実行
    let pool = PgPool::connect_with(settings.with_db())
        .await
        .expect("Failed to connect to test database.");
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the test database.");

    pool
}
