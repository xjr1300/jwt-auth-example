extern crate web_server;

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

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

// Eメールアドレスが一致して、パスワードが誤っている場合に、ユーザーが認証されないことを確認するテスト
#[tokio::test]
#[ignore]
async fn user_unauthorized_when_wrong_password() {
    let app = spawn_web_app().await;
    let data = LoginData {
        email_address: app
            .test_users
            .active_user
            .email_address()
            .value()
            .to_owned(),
        password: "wrong-password".to_owned(),
    };
    let response = app.call_login_api(&data).await;

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

// TODO; Eメールアドレスとパスワードが一致して、アクティブでないユーザーが認証されないことを確認するテスト

// TODO; Eメールアドレスとパスワードが一致して、アクティブなユーザーが認証されることを確認するテスト
