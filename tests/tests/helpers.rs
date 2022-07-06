use std::sync::Arc;

use dotenvy::dotenv;
use once_cell::sync::Lazy;
use reqwest_cookie_store::CookieStoreMutex;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupData {
    pub user_name: String,
    pub email_address: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub email_address: String,
    pub password: String,
}

/// テスト用Webアプリ構造体
pub struct TestWebApp {
    pub settings: Settings,
    pub web_app_address: String,
    pub port: u16,
    pub pool: PgPool,
    pub api_client: reqwest::Client,
    pub cookie_store: Arc<CookieStoreMutex>,
    pub test_users: TestUsers,
}

impl TestWebApp {
    /// ヘルスチェックAPIを呼び出す。
    pub async fn call_health_check_api(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/health_check", self.web_app_address))
            .send()
            .await
            .expect("ヘルスチェックAPIにアクセスできませんでした。")
    }

    /// サインアップAPIを呼び出す。
    pub async fn call_signup_api(&self, data: &SignupData) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/accounts/signup", self.web_app_address))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&data)
            .send()
            .await
            .expect("サインアップAPIにアクセスできませんでした。")
    }

    /// ログインAPIを呼び出す。
    pub async fn call_login_api(&self, data: &LoginData) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/accounts/login", self.web_app_address))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&data)
            .send()
            .await
            .expect("ログインAPIにアクセスできませんでした。")
    }

    /// 保護リソース取得APIを呼び出す。
    pub async fn call_protected_api(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/protected_resource", self.web_app_address))
            .send()
            .await
            .expect("保護リソース取得APIにアクセスできませんでした。")
    }
}

fn get_cookie_store() -> Arc<CookieStoreMutex> {
    Arc::new(CookieStoreMutex::default())
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
    let cookie_store = get_cookie_store();
    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_provider(Arc::clone(&cookie_store))
        // .cookie_storeメソッドを使用すると、reqwest_cookie_storeが有効にならない
        // .cookie_store(false)
        .build()
        .unwrap();

    let web_app = TestWebApp {
        settings: settings.clone(),
        web_app_address: format!("http://localhost:{}", port),
        port,
        pool: get_connection_pool(&settings.db),
        api_client,
        cookie_store: cookie_store.clone(),
        test_users: TestUsers::default(),
    };

    // テストユーザーを登録
    web_app.test_users.store(&web_app.pool).await;

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
