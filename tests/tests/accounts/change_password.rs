use crate::helpers::spawn_web_app;

// ログインしていないユーザーがパスワード変更APIにアクセスできないことを確認するテスト
#[tokio::test]
#[ignore]
async fn cannot_access_change_password() {
    let app = spawn_web_app(true).await;
    let response = app.call_change_password_api().await;
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}
