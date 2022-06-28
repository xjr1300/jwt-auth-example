use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::configurations::{DatabaseSettings, Settings};
use routes::{accounts, health_check};
/// Webアプリ構造体
pub struct WebApp {
    /// Webアプリがリッスンしているポート番号
    port: u16,
    /// Webアプリを提供するサーバー
    server: Server,
}

impl WebApp {
    /// Webアプリを構築する。
    ///
    /// # Arguments
    ///
    /// * `settings` - 設定インスタンス。
    ///
    /// # Returns
    ///
    /// Webアプリインスタンス。
    pub async fn build(settings: Settings) -> Result<Self, anyhow::Error> {
        let pool = get_connection_pool(settings.db);

        let listener = TcpListener::bind(settings.web_app.socket_address())?;
        let port = listener.local_addr().unwrap().port();

        let server = start_web_app(listener, pool).await?;

        Ok(Self { port, server })
    }

    /// Webアプリがリッスンしているポートを返却する。
    ///
    /// # Returns
    ///
    /// Webアプリがリッスンしているポート。
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Webサーバーが終了するまで実行を継続する。
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

/// データベースコネクションプールを構築する。
///
/// # Arguments
///
/// * `settings` - データベース設定。
///
/// # Returns
///
/// データベースコネクションプールインスタンス。
pub fn get_connection_pool(settings: DatabaseSettings) -> PgPool {
    tracing::info!("Connect to database...");
    PgPoolOptions::new().connect_lazy_with(settings.with_db())
}

/// Webアプリを起動する。
///
/// # Argument
///
/// * `listener` - Webアプリのリスナー。
/// * `pool` - データベースコネクションプール。
///
/// # Returns
///
/// Webアプリサーバーインスタンス。
async fn start_web_app(listener: TcpListener, pool: PgPool) -> Result<Server, anyhow::Error> {
    tracing::info!("Startup web app...");
    let server = HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .route("/health_check", web::get().to(health_check::health_check))
            .service(web::scope("/accounts").route("/login", web::post().to(accounts::login)))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
