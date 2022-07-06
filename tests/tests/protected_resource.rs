use crate::helpers::{spawn_web_app, LoginData};

// ログインしたユーザが、保護されたリソースにアクセスできることを確認するテスト。
#[tokio::test]
#[ignore]
async fn can_access_protected_resource() {
    let app = spawn_web_app(true).await;
    let user = &app.test_users.active_user;
    let data = LoginData {
        email_address: user.email_address().value().to_owned(),
        password: app.test_users.active_user_password.clone(),
    };
    let response = app.call_login_api(&data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let response = app.call_protected_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let text = response.text().await.unwrap();
    assert_eq!(text, user.id().value().to_string());
}

// ログインしていないユーザーが、保護されたリソースにアクセスできないことを確認するテスト。
#[tokio::test]
#[ignore]
async fn cannot_access_protected_resource() {
    let app = spawn_web_app(true).await;
    let response = app.call_protected_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

// TODO: アクセストークンの期限が切れていて、リフレッシュトークンが期限内の場合に保護されたリソースにアクセスできることを確認するテストの実装
// ログイン済みのユーザーが、アクセストークンの有効期限が切れていて、リフレッシュトークンが有効期限内の場合に、保護されたリソースにアクセスできる
// ことを確認するテストを実装する。また、ブラウザにクッキーとして保存されたアクセストークンとリフレッシュトークンが、変更されていることを確認する。

// TODO: リフレッシュトークンの期限が切れていて、保護されたリソースにアクセスできないことを確認するテストの実装
// ログイン済みのユーザーが、リフレッシュトークンの有効期限が切れていて、保護されたリソースにアクセスできないことを確認するテストの実装をる。また、
// ブラウザのセッションデータクッキーが削除されていることを確認する。
