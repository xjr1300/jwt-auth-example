extern crate web_server;

mod helpers;

#[tokio::test]
#[ignore]
async fn health_check_works() {
    let app = helpers::spawn_web_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health_check", app.web_app_address))
        .send()
        .await
        .expect("ヘルスチェックに失敗しました。");

    assert!(response.status().is_success());
}
