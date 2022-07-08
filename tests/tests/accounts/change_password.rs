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

/// 現在のパスワードが間違っている場合に、パスワードを変更できないことを確認するテスト
#[tokio::test]
#[ignore]
async fn cannot_change_password_if_incorrect_current_password() {
    // ログイン
    let app = spawn_web_app(true).await;
    let login_data = app.active_user_login_data();
    let _ = app.call_login_api(&login_data).await;

    // パスワードを変更できないことを確認
    let mut change_password_data = app.change_password_data();
    change_password_data.current_password = "S5yN@]5E6-LV".to_owned();
    let response = app.call_change_password_api(&change_password_data).await;
    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

    // ログアウト
    let _ = app.call_logout_api().await;

    // 同じパスワード（変更されていないパスワード）で、ログインできることを確認
    let login_data = app.active_user_login_data();
    let response = app.call_login_api(&login_data).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK)
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
    // セッションIDとトークンを記憶
    let session_id = app.get_session_id().unwrap();
    let (access_token, refresh_token) = app.get_token_values();

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
    // セッションIDとトークンが変わっていることを確認
    let session_id_2nd = app.get_session_id().unwrap();
    let (access_token_2nd, refresh_token_2nd) = app.get_token_values();
    assert!(session_id != session_id_2nd);
    assert!(access_token != access_token_2nd);
    assert!(refresh_token != refresh_token_2nd);
}
