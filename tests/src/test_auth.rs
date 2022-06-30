extern crate web_server;

use serde::{Deserialize, Serialize};

mod test_helpers;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginData {
    email_address: String,
    password: String,
}

/// 登録されていないユーザーが認証されないことを確認するテスト
#[tokio::test]
#[ignore]
async fn test_anonymous_user_unauthorized() {
    let app = test_helpers::spawn_web_app().await;
    let data = LoginData {
        email_address: "anonymous@example.com".to_owned(),
        password: "anonymous-password".to_owned(),
    };
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/auth/login", app.web_app_address))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&data)
        .send()
        .await
        .expect("ログインAPIにアクセスできませんでした。");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

// TODO; Eメールアドレスが一致して、パスワードが異なるユーザーが認証されないことを確認するテスト

// TODO; Eメールアドレスとパスワードが一致して、アクティブでないユーザーが認証されないことを確認するテスト

// TODO; Eメールアドレスとパスワードが一致して、アクティブなユーザーが認証されることを確認するテスト
