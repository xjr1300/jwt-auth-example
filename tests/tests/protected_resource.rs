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

/// アクセストークンが失効していて、リフレッシュトークンが期限内の場合に、保護されたリソースにアクセスできることを確認するテスト
///
/// ログイン済みのユーザーのアクセストークンの有効期限が切れていて、リフレッシュトークンが有効期限内の場合に、
/// 保護されたリソースにアクセスできることを確認する。また、ブラウザにクッキーとして保存されたアクセストークン
/// とリフレッシュトークンが、ログインしたときと2回目に保護されたリソースにアクセスしたときで、異なることを
/// 確認する。
// FIXME: 単体で本テストを実行するとパスするが、他の統合テストと一緒に実行するとパスしない。
// 原因を特定して修正すること。
// 修正するまで、統合テストは`./scripts/integration_tests.sh`スクリプトで実行すること。
#[tokio::test]
#[ignore]
async fn can_access_protected_resource_at_within_expiration_of_refresh_token() {
    // 環境変数を設定して、テスト用Webアプリを起動
    dotenvy::dotenv().ok();
    std::env::set_var("ACCESS_TOKEN_SECONDS", "1");
    let app = spawn_web_app(false).await;
    let user = &app.test_users.active_user;
    // ログイン
    let data = LoginData {
        email_address: user.email_address().value().to_owned(),
        password: app.test_users.active_user_password.clone(),
    };
    let response = app.call_login_api(&data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    // アクセストークンとリフレッシュトークンを取得
    let (access_token, refresh_token) = app.get_token_values();
    // 保護されたリソースにアクセス
    let response = app.call_protected_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let text = response.text().await.unwrap();
    assert_eq!(text, user.id().value().to_string());

    // 2秒待機
    std::thread::sleep(std::time::Duration::from_secs(2));

    // 再度、保護されたリソースにアクセス
    let response = app.call_protected_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let text = response.text().await.unwrap();
    assert_eq!(text, user.id().value().to_string());
    // 再度、アクセストークンとリフレッシュトークンを取得
    let (access_token_2nd, refresh_token_2nd) = app.get_token_values();
    // アクセストークンとリフレッシュトークンが変更されていることを確認
    assert!(access_token != access_token_2nd);
    assert!(refresh_token != refresh_token_2nd);
}

// ログイン済みのユーザーが、リフレッシュトークンが失効したとき、保護されたリソースにアクセスできないことを確認するテスト
// FIXME: 単体で本テストを実行するとパスするが、他の統合テストと一緒に実行するとパスしない。
// 原因を特定して修正すること。
// 修正するまで、統合テストは`./scripts/integration_tests.sh`スクリプトで実行すること。
#[tokio::test]
#[ignore]
async fn cannot_access_protected_resource_at_expired_expiration_of_refresh_token() {
    // 環境変数を設定して、テスト用Webアプリを起動
    dotenvy::dotenv().ok();
    std::env::set_var("ACCESS_TOKEN_SECONDS", "1");
    std::env::set_var("REFRESH_TOKEN_SECONDS", "1");
    let app = spawn_web_app(false).await;
    let user = &app.test_users.active_user;
    // ログイン
    let data = LoginData {
        email_address: user.email_address().value().to_owned(),
        password: app.test_users.active_user_password.clone(),
    };
    let response = app.call_login_api(&data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    // 保護されたリソースにアクセス
    let response = app.call_protected_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let text = response.text().await.unwrap();
    assert_eq!(text, user.id().value().to_string());

    // 2秒待機
    std::thread::sleep(std::time::Duration::from_secs(2));

    // 再度、保護されたリソースにアクセス
    let response = app.call_protected_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    // FIXME: トークンを記録したクッキーが削除されていることを確認
    // // 再度、アクセストークンとリフレッシュトークンを取得
    // let (access_token_2nd, refresh_token_2nd) = app.get_token_values();
    // // アクセストークンとリフレッシュトークンが変更されていることを確認
    // assert!(access_token_2nd.is_none());
    // assert!(refresh_token_2nd.is_none());
}
