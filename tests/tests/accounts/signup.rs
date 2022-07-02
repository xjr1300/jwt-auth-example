extern crate web_server;

use serde::Deserialize;
use time::OffsetDateTime;

use crate::helpers::{spawn_web_app, SignupData};

#[derive(Debug, Deserialize)]
struct PartialUser {
    user_name: String,
    email_address: String,
    is_active: bool,
    last_logged_in: Option<OffsetDateTime>,
    created_at: Option<OffsetDateTime>,
    updated_at: Option<OffsetDateTime>,
}

/// サインアップできることを確認するテスト
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
    let user: PartialUser = serde_json::from_value(response.json().await.unwrap()).unwrap();
    assert_eq!(user.user_name, data.user_name);
    assert_eq!(user.email_address, data.email_address);
    assert!(user.is_active);
    assert!(user.last_logged_in.is_none());
    assert!(user.created_at.is_some());
    assert!(user.updated_at.is_some());
}
