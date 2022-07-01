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

    // TODO: クッキーにセッションデータが記録されているか確認

    // TODO: Redisにセッションデータが記録されているか確認
}
