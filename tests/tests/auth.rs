extern crate web_server;

use configurations::{SessionCookieSettings, Settings};
use cookie_store::{Cookie, CookieExpiration};

use crate::helpers::{spawn_web_app, LoginData};

/// 登録されていないユーザーが認証されないことを確認するテスト
#[tokio::test]
#[ignore]
async fn anonymous_user_unauthorized() {
    let app = spawn_web_app().await;
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
    let app = spawn_web_app().await;
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
    let app = spawn_web_app().await;
    let user = &app.test_users.non_active_user;
    let data = LoginData {
        email_address: user.email_address().value().to_owned(),
        password: app.test_users.non_active_user_password.clone(),
    };
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

// TODO; Eメールアドレスとパスワードが正しくて、アクティブなユーザーが認証されることを確認するテスト
#[tokio::test]
#[ignore]
async fn active_user_authorized() {
    let app = spawn_web_app().await;
    let user = &app.test_users.active_user;
    let data = LoginData {
        email_address: user.email_address().value().to_owned(),
        password: app.test_users.active_user_password.clone(),
    };
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
    let session_id = {
        let Settings {
            ref session_cookie, ..
        } = app.settings;
        let store = app.cookie_store.lock().unwrap();
        // セッションID
        let session_id_cookie = store.get("localhost", "/", &session_cookie.session_id_cookie_name);
        assert_cookie(session_id_cookie.unwrap(), session_cookie);

        // トークン
        let cookie_names = vec!["access_token", "refresh_token"];
        for cookie_name in cookie_names {
            let cookie = store.get("localhost", "/", cookie_name);
            assert!(
                cookie.is_some(),
                "クッキー{}が記録されていません。",
                cookie_name
            );
            assert_cookie(cookie.unwrap(), session_cookie);
        }

        session_id_cookie.unwrap().value().to_owned()
    };

    // TODO: Redisにセッションデータが記録されているか確認
}
