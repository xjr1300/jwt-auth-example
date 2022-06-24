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

impl EnvValues {
    /// template1データベースに接続するオプションを返却する。
    ///
    /// # Returns
    ///
    /// データベース接続オプションインスタンス。
    pub fn database_connect_option_without_database(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .username(&self.postgres_user_name)
            .password(self.postgres_user_password.expose_secret())
            .host(&self.postgres_host)
            .port(self.postgres_port)
    }

    /// Webアプリ用のデータベースに接続するオプションを返却する。
    ///
    /// # Returns
    ///
    /// データベース接続オプションインスタンス。
    pub fn database_connect_option_with_database(&self) -> PgConnectOptions {
        let mut options = self
            .database_connect_option_without_database()
            .database(&self.postgres_database_name);
        options.log_statements(tracing::log::LevelFilter::Trace);

        options
    }
}

/// 環境変数
pub static ENV_VALUES: Lazy<EnvValues> = Lazy::new(|| {
    dotenv().ok();

    EnvValues {
        // Rust
        rust_log: env::var("RUST_LOG").expect("環境変数にRUST_LOGが設定されていません。"),
        // データベース
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
