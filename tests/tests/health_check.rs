extern crate web_server;

use crate::helpers::spawn_web_app;

/// ヘルスチェックが正常に動作するか確認するテスト
#[tokio::test]
#[ignore]
async fn health_check() {
    let app = spawn_web_app(true).await;
    let response = app.call_health_check_api().await;
    assert!(
        response.status().is_success(),
        "ヘルスチェックAPIが20x以外を返却しました。"
    );
    let body = response.text().await.unwrap();
    assert_eq!(
        body, "Are you ready?",
        "ヘルスチェックAPIが返却したボディが想定と一致しません。"
    )
}
