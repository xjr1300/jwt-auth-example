use crate::helpers::spawn_web_app;

/// ログインしていないユーザーがパスワード変更APIにアクセスできないことを確認するテスト
#[tokio::test]
#[ignore]
async fn cannot_access_change_password() {
    let app = spawn_web_app(true).await;
    let data = app.change_password_data();
    let response = app.call_change_password_api(&data).await;
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

/// ログインしているユーザーがパスワードを変更できることを確認するテスト
#[tokio::test]
#[ignore]
async fn can_change_password() {
    // ログイン
    let app = spawn_web_app(true).await;
    let mut login_data = app.active_user_login_data();
    let response = app.call_login_api(&login_data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // パスワードを変更
    let change_password_data = app.change_password_data();
    let response = app.call_change_password_api(&change_password_data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // 保護されたリソースにアクセスできないことを確認（ログアウトしている）
    let response = app.call_protected_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    // 古いパスワードでログインできないことを確認
    let response = app.call_login_api(&login_data).await;
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    // 新しいパスワードでログインできることを確認
    login_data.password = change_password_data.new_password.clone();
    let response = app.call_login_api(&login_data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}
