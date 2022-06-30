use std::net::TcpListener;

use actix_session::{storage::RedisSessionStore, SessionLength, SessionMiddleware};
use actix_web::{cookie::Key, dev::Server, web, App, HttpServer};
use secrecy::ExposeSecret;
use sqlx::{postgres::PgPoolOptions, PgPool};

use routes::{auth, health_check};

use configurations::{DatabaseSettings, Settings};

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
        let Settings {
            web_app,
            session_cookie,
            tokens,
            session_store,
            db,
            ..
        } = settings;
        let pool = web::Data::new(get_connection_pool(&db));

        let listener = TcpListener::bind(web_app.socket_address())?;
        let port = listener.local_addr().unwrap().port();

        let store = RedisSessionStore::new(session_store.uri.expose_secret()).await?;
        let store_key = Key::from(session_store.key.expose_secret().as_bytes());

        tracing::info!("Startup web app...");
        let server = HttpServer::new(move || {
            App::new()
                .wrap(
                    SessionMiddleware::builder(store.clone(), store_key.clone())
                        .session_length(SessionLength::BrowserSession {
                            state_ttl: Some(tokens.access_token_duration),
                        })
                        .cookie_http_only(true)
                        .cookie_same_site(session_cookie.same_site)
                        .cookie_secure(session_cookie.secure)
                        .build(),
                )
                .app_data(pool.clone())
                .route("/health_check", web::get().to(health_check::health_check))
                .service(web::scope("/auth").route("/login", web::post().to(auth::login)))
        })
        .listen(listener)?
        .run();

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
pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    tracing::info!("Connect to database...");
    PgPoolOptions::new().connect_lazy_with(settings.with_db())
}
