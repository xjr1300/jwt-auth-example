use std::env;

use actix_web::cookie::{time::Duration, SameSite};
use anyhow::bail;
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use secrecy::{ExposeSecret, Secret};
use sqlx::{postgres::PgConnectOptions, ConnectOptions};

/// 設定構造体
#[derive(Debug, Clone)]
pub struct Settings {
    /// Rust設定
    pub rust_log: String,
    /// Webアプリ設定
    pub web_app: WebAppSettings,
    /// セッション設定
    pub session_cookie: SessionCookieSettings,
    /// トークン設定
    pub tokens: TokensSettings,
    /// セッションストア設定
    pub session_store: SessionStoreSettings,
    /// データベース設定
    pub db: DatabaseSettings,
}

/// 環境変数から設定を取得する。
///
/// # Returns
///
/// 設定インスタンス。
pub fn get_settings() -> Settings {
    Settings {
        rust_log: ENV_VALUES.rust_log.clone(),
        web_app: WebAppSettings::default(),
        session_cookie: SessionCookieSettings::default(),
        tokens: TokensSettings::default(),
        session_store: SessionStoreSettings::default(),
        db: DatabaseSettings::default(),
    }
}

fn str_to_same_site(value: &str) -> anyhow::Result<SameSite> {
    match value {
        "none" => Ok(SameSite::None),
        "lax" => Ok(SameSite::Lax),
        "strict" => Ok(SameSite::Strict),
        _ => bail!("文字列からSameSiteを取得できません。"),
    }
}

/// 環境変数構造体
pub struct EnvValues {
    pub rust_log: String,

    pub web_app_host: String,
    pub web_app_port: u16,

    pub session_cookie_secure: bool,
    pub session_cookie_same_site: SameSite,

    pub token_secret_key: Secret<String>,
    pub access_token_duration: Duration,
    pub refresh_token_duration: Duration,

    pub session_store_uri: Secret<String>,
    pub session_store_key: Secret<String>,

    pub postgres_user_name: String,
    pub postgres_user_password: Secret<String>,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_database_name: String,
}

fn string_from_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("環境変数に{}が設定されていません。", key))
}

fn u16_from_env(key: &str) -> u16 {
    env::var(key)
        .unwrap_or_else(|_| panic!("環境変数に{}が設定されていません。", key))
        .parse()
        .unwrap_or_else(|_| panic!("環境変数{}を数値として認識できません。", key))
}

fn bool_from_env(key: &str) -> bool {
    env::var(key)
        .unwrap_or_else(|_| panic!("環境変数に{}が設定されていません。", key))
        .parse()
        .unwrap_or_else(|_| panic!("環境変数{}を論理値として認識できません。", key))
}

fn same_site_from_env(key: &str) -> SameSite {
    str_to_same_site(
        &env::var(key).unwrap_or_else(|_| panic!("環境変数に{}が設定されていません。", key)),
    )
    .unwrap_or_else(|_| panic!("環境変数{}をSameSiteとして認識できません。", key))
}

fn seconds_from_env(key: &str) -> Duration {
    Duration::seconds(
        env::var(key)
            .unwrap_or_else(|_| panic!("環境変数に{}が設定されていません。", key))
            .parse()
            .unwrap_or_else(|_| panic!("環境変数{}を秒数として認識できません。", key)),
    )
}

/// 環境変数
pub static ENV_VALUES: Lazy<EnvValues> = Lazy::new(|| {
    dotenv().ok();

    EnvValues {
        // Rust設定
        rust_log: string_from_env("RUST_LOG"),

        // Webアプリ設定
        web_app_host: string_from_env("WEB_APP_HOST"),
        web_app_port: u16_from_env("WEB_APP_PORT"),

        // セッション設定
        session_cookie_secure: bool_from_env("SESSION_COOKIE_SECURE"),
        session_cookie_same_site: same_site_from_env("SESSION_COOKIE_SAME_SITE"),

        // セッションストア設定
        session_store_uri: Secret::new(string_from_env("SESSION_STORE_URI")),
        session_store_key: Secret::new(string_from_env("SESSION_STORE_KEY")),

        // トークン設定
        token_secret_key: Secret::new(string_from_env("TOKEN_SECRET_KEY")),
        access_token_duration: seconds_from_env("ACCESS_TOKEN_SECONDS"),
        refresh_token_duration: seconds_from_env("REFRESH_TOKEN_SECONDS"),

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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct SessionCookieSettings {
    pub secure: bool,
    pub same_site: SameSite,
}

impl SessionCookieSettings {
    pub fn default() -> Self {
        Self {
            secure: ENV_VALUES.session_cookie_secure,
            same_site: ENV_VALUES.session_cookie_same_site,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokensSettings {
    pub secret_key: Secret<String>,
    pub access_token_duration: Duration,
    pub refresh_token_duration: Duration,
}

impl TokensSettings {
    pub fn default() -> Self {
        Self {
            secret_key: ENV_VALUES.token_secret_key.clone(),
            access_token_duration: ENV_VALUES.access_token_duration,
            refresh_token_duration: ENV_VALUES.refresh_token_duration,
        }
    }
}

/// SessionStore設定構造体
#[derive(Debug, Clone)]
pub struct SessionStoreSettings {
    pub uri: Secret<String>,
    pub key: Secret<String>,
}

impl SessionStoreSettings {
    pub fn default() -> Self {
        Self {
            uri: ENV_VALUES.session_store_uri.clone(),
            key: ENV_VALUES.session_store_key.clone(),
        }
    }
}

/// データベース設定構造体
#[derive(Debug, Clone)]
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
