use std::env;

use dotenvy::dotenv;
use once_cell::sync::Lazy;
use secrecy::{ExposeSecret, Secret};
use sqlx::{postgres::PgConnectOptions, ConnectOptions};

/// 環境変数構造体
#[derive(Debug)]
pub struct EnvValues {
    /// RUST_LOG
    pub rust_log: String,

    /// Webアプリホスト名
    pub web_app_host: String,
    /// Webアプリポート番号
    pub web_app_port: u16,

    /// PostgreSQLユーザー名
    pub postgres_user_name: String,
    /// PostgreSQLパスワード
    pub postgres_user_password: Secret<String>,
    /// PostgreSQLホスト名
    pub postgres_host: String,
    /// PostgreSQLポート番号
    pub postgres_port: u16,
    /// PostgreSQLデータベース名
    pub postgres_database_name: String,
}

/// 環境変数
pub static ENV_VALUES: Lazy<EnvValues> = Lazy::new(|| {
    dotenv().ok();

    EnvValues {
        // Rust設定
        rust_log: env::var("RUST_LOG").expect("環境変数にRUST_LOGが設定されていません。"),

        // Webアプリ設定
        web_app_host: env::var("WEB_APP_HOST")
            .expect("環境変数にWEB_APP_HOSTが設定されていません。"),
        web_app_port: env::var("WEB_APP_PORT")
            .expect("環境変数にWEB_APP_PORTが設定されていません。")
            .parse::<u16>()
            .expect("環境変数WEB_APP_PORTを数値として認識できません。"),

        // データベース設定
        postgres_user_name: env::var("POSTGRES_USER_NAME")
            .expect("環境変数にPOSTGRES_USER_NAMEが設定されていません。"),
        postgres_user_password: Secret::new(
            env::var("POSTGRES_USER_PASSWORD")
                .expect("環境変数にPOSTGRES_USER_PASSWORDが設定されていません。"),
        ),
        postgres_host: env::var("POSTGRES_HOST")
            .expect("環境変数にPOSTGRES_HOSTが設定されていません。"),
        postgres_port: env::var("POSTGRES_PORT")
            .expect("環境変数にPOSTGRES_PORTが設定されていません。")
            .parse::<u16>()
            .expect("環境変数POSTGRES_PORTを数値として認識できません。"),
        postgres_database_name: env::var("POSTGRES_DATABASE_NAME")
            .expect("環境変数にPOSTGRES_DATABASE_NAMEが設定されてません。"),
    }
});

/// Webアプリ設定構造体
pub struct WebAppSettings {
    pub host: String,
    pub port: u16,
}

impl WebAppSettings {
    /// 環境変数からWebアプリ設定を構築する。
    ///
    /// # Returns
    ///
    /// Webアプリ設定インスタンス。
    pub fn default() -> Self {
        Self {
            host: ENV_VALUES.web_app_host.clone(),
            port: ENV_VALUES.web_app_port,
        }
    }

    /// Webアプリがバインドするソケットアドレスを返却する。
    ///
    /// # Returns
    ///
    /// Webアプリがバインドするソケットアドレス。
    pub fn socket_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// データベース設定構造体
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub database_name: String,
}

impl DatabaseSettings {
    /// 環境変数からデータベース設定を構築する。
    ///
    /// # Returns
    ///
    /// データベース設定インスタンス。
    pub fn default() -> Self {
        Self {
            username: ENV_VALUES.postgres_user_name.clone(),
            password: ENV_VALUES.postgres_user_password.clone(),
            host: ENV_VALUES.postgres_host.clone(),
            port: ENV_VALUES.postgres_port,
            database_name: ENV_VALUES.postgres_database_name.clone(),
        }
    }

    /// template1データベースに接続するオプションを返却する。
    ///
    /// # Returns
    ///
    /// データベース接続オプションインスタンス。
    pub fn without_db(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .username(&self.username)
            .password(self.password.expose_secret())
            .host(&self.host)
            .port(self.port)
    }

    /// Webアプリ用のデータベースに接続するオプションを返却する。
    ///
    /// # Returns
    ///
    /// データベース接続オプションインスタンス。
    pub fn with_db(&self) -> PgConnectOptions {
        let mut options = self.without_db().database(&self.database_name);
        options.log_statements(tracing::log::LevelFilter::Trace);

        options
    }
}
