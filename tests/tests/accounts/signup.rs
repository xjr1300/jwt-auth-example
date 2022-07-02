extern crate web_server;

use crate::helpers::{spawn_web_app, SignupData};

/// [temporary] サインアップAPIにアクセスできることを確認するテスト
#[tokio::test]
#[ignore]
async fn signup() {
    let app = spawn_web_app().await;
    let data = SignupData {
        user_name: "foo".to_owned(),
        email_address: "foo@example.com".to_owned(),
        // cspell:disable-next-line
        password: "tOC8pHh:K/-G".to_owned(),
    };
    let response = app.call_signup_api(&data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}
