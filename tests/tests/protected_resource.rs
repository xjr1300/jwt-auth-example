use crate::helpers::{spawn_web_app, LoginData};

// ログインしたユーザが、保護されたリソースにアクセスできることを確認するテスト。
#[tokio::test]
#[ignore]
async fn can_access_protected_resource() {
    let app = spawn_web_app().await;
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

