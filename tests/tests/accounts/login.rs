extern crate web_server;

use configurations::session::{ACCESS_TOKEN_COOKIE_NAME, REFRESH_TOKEN_COOKIE_NAME};
use configurations::{SessionCookieSettings, Settings};
use cookie_store::{Cookie, CookieExpiration};
// use redis::Commands;
// use secrecy::ExposeSecret;

use crate::helpers::{spawn_web_app, LoginData};

/// 登録されていないユーザーが認証されないことを確認するテスト
#[tokio::test]
#[ignore]
async fn anonymous_user_unauthorized() {
    let app = spawn_web_app(true).await;
    let data = LoginData {
        email_address: "anonymous@example.com".to_owned(),
        password: "anonymous-password".to_owned(),
    };
    let response = app.call_login_api(&data).await;
    // 401 Unauthorizedが返却されるか確認
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

// Eメールアドレスが正しくて、パスワードが誤っている場合に、ユーザーが認証されないことを確認するテスト
#[tokio::test]
#[ignore]
async fn user_unauthorized_when_wrong_password() {
    let app = spawn_web_app(true).await;
    let user = &app.test_users.active_user;
    let data = LoginData {
        email_address: user.email_address().value().to_owned(),
        password: "wrong-password".to_owned(),
    };
    let response = app.call_login_api(&data).await;
    // 401 Unauthorizedが返却されるか確認
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

// Eメールアドレスとパスワードが正しくて、アクティブでないユーザーが認証されないことを確認するテスト
#[tokio::test]
#[ignore]
async fn non_active_user_unauthorized() {
    let app = spawn_web_app(true).await;
    let data = app.non_active_user_login_data();
    let response = app.call_login_api(&data).await;
    // 401 Unauthorizedが返却されるか確認
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

fn assert_cookie(cookie: &Cookie, settings: &SessionCookieSettings) {
    assert!(cookie.http_only().unwrap());
    if !cookie.secure().is_none() {
        assert_eq!(cookie.secure().unwrap(), settings.secure);
    } else {
        assert!(!settings.secure);
    }
    assert_eq!(cookie.same_site().unwrap(), settings.same_site);
    assert_eq!(cookie.expires, CookieExpiration::SessionEnd);
}

// Eメールアドレスとパスワードが正しくて、アクティブなユーザーが認証されることを確認するテスト
#[tokio::test]
#[ignore]
async fn active_user_authorized() {
    let app = spawn_web_app(true).await;
    let Settings {
        ref session_cookie,
        // ref session_store,
        ..
    } = app.settings;
    let user = &app.test_users.active_user;
    let data = app.active_user_login_data();
    let response = app.call_login_api(&data).await;
    // 200 OKが返却されるか確認
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // 最終ログイン日時が更新されているか確認
    let result = sqlx::query!(
        r#"
            SELECT last_logged_in
            FROM users
            WHERE id = $1
        "#,
        user.id().value(),
    )
    .fetch_one(&app.pool)
    .await
    .expect("データベースからユーザーを取得できませんでした。");
    assert!(result.last_logged_in.is_some());

    // クッキーにセッションデータが記録されているか確認
    let _session_id = {
        let store = app.cookie_store.lock().unwrap();
        // セッションID
        let session_id_cookie = store.get("localhost", "/", &session_cookie.session_id_cookie_name);
        assert_cookie(session_id_cookie.unwrap(), session_cookie);

        // トークン
        let cookie_names = vec![ACCESS_TOKEN_COOKIE_NAME, REFRESH_TOKEN_COOKIE_NAME];
        for cookie_name in cookie_names {
            let cookie = store.get("localhost", "/", cookie_name);
            assert!(
                cookie.is_some(),
                "クッキーに{}が記録されていません。",
                cookie_name
            );
            assert_cookie(cookie.unwrap(), session_cookie);
        }

        session_id_cookie.unwrap().value().to_owned()
    };

    // FIXME: Redisにセッションデータが記録されているか確認
    // actix-sessionは、actix-sessionが生成したセッションIDを加工した文字列を保存ことをブラウザに指示しているように見える。
    // また、加工する際に使用する鍵は、本プログラムにおいては環境変数SESSION_STORE_KEY及び`settings.session_store.keyが
    // 該当する。
    // https://github.com/actix/actix-extras/blob/d11a2723841d1eb376c1fbfbda8f432ad2d7d9c6/actix-session/src/middleware.rs#L601
    // 暗号化したセッションIDを複合する方法を把握した後に、本テストを実装すること。
    // https://github.com/actix/actix-extras/blob/d11a2723841d1eb376c1fbfbda8f432ad2d7d9c6/actix-session/src/middleware.rs#L522
    // let client = redis::Client::open(session_store.uri.expose_secret().as_str()).unwrap();
    // let mut conn = client.get_connection().unwrap();
    // 下の行で、セッションデータの取得を試みるが、Redisはnilを返却する。
    // let _session_data: String = conn.get(session_id).unwrap();
}
