extern crate web_server;

use serde::Deserialize;
use time::OffsetDateTime;

use crate::helpers::{spawn_web_app, SignupData, TestWebApp};

#[derive(Debug, Deserialize)]
struct PartialUser {
    user_name: String,
    email_address: String,
    is_active: bool,
    last_logged_in: Option<OffsetDateTime>,
    created_at: Option<OffsetDateTime>,
    updated_at: Option<OffsetDateTime>,
}

const USER_NAME: &str = "foo";
const EMAIL_ADDRESS: &str = "foo@example.com";
// cspell:disable-next-line
const PASSWORD: &str = "tOC8pHh:K/-G";

/// 固定したユーザーを登録する。
async fn signup_fixed_user(app: &TestWebApp) -> reqwest::Response {
    let data = SignupData {
        user_name: USER_NAME.to_owned(),
        email_address: EMAIL_ADDRESS.to_owned(),
        password: PASSWORD.to_owned(),
    };

    app.call_signup_api(&data).await
}

/// サインアップできることを確認するテスト
#[tokio::test]
#[ignore]
async fn signup() {
    let app = spawn_web_app().await;
    let response = signup_fixed_user(&app).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let user: PartialUser = serde_json::from_value(response.json().await.unwrap()).unwrap();
    assert_eq!(user.user_name, USER_NAME);
    assert_eq!(user.email_address, EMAIL_ADDRESS);
    assert!(user.is_active);
    assert!(user.last_logged_in.is_none());
    assert!(user.created_at.is_some());
    assert!(user.updated_at.is_some());
}

/// 同じEメールアドレスを持つユーザーが登録されているときに、登録できないことを確認するテスト
#[tokio::test]
#[ignore]
async fn cannot_signup_same_email_address() {
    let app = spawn_web_app().await;
    // 同じEメールアドレスを持つユーザーを2回登録
    let _ = signup_fixed_user(&app).await;
    let response = signup_fixed_user(&app).await;
    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
}
