extern crate web_server;

mod test_helpers;

/// ヘルスチェックが正常に動作するか確認するテスト
#[tokio::test]
#[ignore]
async fn test_health_check() {
    let app = test_helpers::spawn_web_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health_check", app.web_app_address))
        .send()
        .await
        .expect("ヘルスチェックAPIにアクセスできませんでした。");
    assert!(response.status().is_success(), "ヘルスチェックAPIが20x以外を返却しました。");

    let body = response.text().await.unwrap();
    assert_eq!(body, "Are you ready?", "ヘルスチェックAPIが返却したボディが想定と一致しません。")
}
