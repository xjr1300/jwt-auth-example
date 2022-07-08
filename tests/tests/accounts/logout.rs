use crate::helpers::spawn_web_app;

// ログインしているユーザーがログアウトできることを確認するテスト
#[tokio::test]
#[ignore]
async fn logout() {
    // ログイン
    let app = spawn_web_app(true).await;
    let data = app.active_user_login_data();
    let response = app.call_login_api(&data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // ログアウト
    let response = app.call_logout_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // 保護されたリソースにアクセスできないことを確認
    let response = app.call_protected_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    // FIXME: トークンを記録したクッキーが削除されていることを確認
    // use actix_web::cookie::time::Duration;
    // use configurations::session::{ACCESS_TOKEN_COOKIE_NAME, REFRESH_TOKEN_COOKIE_NAME};
    // let store = app.cookie_store.lock().unwrap();
    // let access_token_cookie = store.get("localhost", "/", ACCESS_TOKEN_COOKIE_NAME);
    // let refresh_token_cookie = store.get("localhost", "/", REFRESH_TOKEN_COOKIE_NAME);
    // let cookies = vec![access_token_cookie, refresh_token_cookie];
    // for cookie in cookies {
    //     match cookie {
    //         Some(cookie) => {
    //             // assert_eq!(cookie.value(), "");
    //             assert!(cookie.max_age() == Some(Duration::ZERO) || cookie.max_age().is_none());
    //         }
    //         None => (),
    //     }
    // }
}

// ログインしていないユーザーがログアウトできないことを確認するテスト
#[tokio::test]
#[ignore]
async fn cannot_logout() {
    // ログアウト
    let app = spawn_web_app(true).await;
    let response = app.call_logout_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}
