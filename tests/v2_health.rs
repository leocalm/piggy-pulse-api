mod common;

use common::test_client;
use rocket::http::Status;
use serde_json::Value;

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_health_ok() {
    let client = test_client().await;

    let resp = client.get(format!("{}/health", common::V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_health_response_shape() {
    let client = test_client().await;

    let resp = client.get(format!("{}/health", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["status"].is_string(), "expected status field");
    assert!(body["database"].is_string(), "expected database field");
}
