use std::sync::Arc;

use cookie_store::Cookie;
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use reqwest_cookie_store::CookieStoreMutex;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPool, Connection, Executor, PgConnection};
use uuid::Uuid;

use configurations::session::{ACCESS_TOKEN_COOKIE_NAME, REFRESH_TOKEN_COOKIE_NAME};
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordData {
    pub current_password: String,
    pub new_password: String,
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

    pub fn active_user_login_data(&self) -> LoginData {
        LoginData {
            email_address: self
                .test_users
                .active_user
                .email_address()
                .value()
                .to_owned(),
            password: self.test_users.active_user_password.clone(),
        }
    }

    pub fn non_active_user_login_data(&self) -> LoginData {
        LoginData {
            email_address: self
                .test_users
                .non_active_user
                .email_address()
                .value()
                .to_owned(),
            password: self.test_users.active_user_password.clone(),
        }
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

    /// ログアウトAPIを呼び出す。
    pub async fn call_logout_api(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/accounts/logout", self.web_app_address))
            .send()
            .await
            .expect("ログアウトAPIにアクセスできませんでした。")
    }

    /// 保護リソース取得APIを呼び出す。
    pub async fn call_protected_api(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/protected_resource", self.web_app_address))
            .send()
            .await
            .expect("保護リソース取得APIにアクセスできませんでした。")
    }

    pub fn change_password_data(&self) -> ChangePasswordData {
        ChangePasswordData {
            current_password: self.test_users.active_user_password.clone(),
            new_password: "6i8TR:6Al@.d".to_owned(),
        }
    }

    /// パスワード変更APIを呼び出す。
    pub async fn call_change_password_api(&self, data: &ChangePasswordData) -> reqwest::Response {
        self.api_client
            .post(&format!(
                "{}/accounts/change_password",
                self.web_app_address
            ))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&data)
            .send()
            .await
            .expect("パスワード変更APIにアクセスできませんでした。")
    }

    /// アクセストークンとリフレッシュトークンを取得する。
    pub fn get_token_values(&self) -> (Option<String>, Option<String>) {
        let store = self.cookie_store.lock().unwrap();
        let access_token_cookie = store.get("localhost", "/", ACCESS_TOKEN_COOKIE_NAME);
        let refresh_token_cookie = store.get("localhost", "/", REFRESH_TOKEN_COOKIE_NAME);

        (
            get_cookie_value(access_token_cookie),
            get_cookie_value(refresh_token_cookie),
        )
    }
}

fn get_cookie_store() -> Arc<CookieStoreMutex> {
    Arc::new(CookieStoreMutex::default())
}

fn get_cookie_value(cookie: Option<&Cookie>) -> Option<String> {
    match cookie {
        Some(cookie) => Some(cookie.value().to_owned()),
        None => None,
    }
}

/// テスト用Webアプリを生成する。
///
/// `is_dotenv`を`false`にすることで、テストコードで`dotenv().ok()`を実行した後、環境変数を設定することで、
/// システム設定をカスタマイズできる。
///
/// # Arguments
///
/// * `is_dotenv` - `true`の場合`dotenv().ok()`を実行して、`false`の場合は実行しない。
pub async fn spawn_web_app(is_dotenv: bool) -> TestWebApp {
    if is_dotenv {
        dotenv().ok();
    }

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
