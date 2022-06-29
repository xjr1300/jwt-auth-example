extern crate web_server;

mod helpers;

/// ヘルスチェックが正常に動作するか確認するテスト
#[tokio::test]
#[ignore]
async fn test_health_check() {
    let app = helpers::spawn_web_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health_check", app.web_app_address))
        .send()
        .await
        .expect("ヘルスチェックに失敗しました。");
    assert!(response.status().is_success());

    let body = response.text().await.unwrap();
    assert_eq!(body, "Are you ready?")
}
