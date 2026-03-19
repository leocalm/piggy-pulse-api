mod common;

use common::{V2_BASE, test_client};
use rocket::http::Status;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /unlock?token=...
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unlock_invalid_token_returns_400() {
    let client = test_client().await;

    let resp = client.get(format!("{}/unlock?token=invalid-token-value-12345", V2_BASE)).dispatch().await;

    // Invalid token should return 400
    assert_eq!(resp.status(), Status::BadRequest);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["message"].is_string(), "error response must have message field");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unlock_missing_token_returns_400() {
    let client = test_client().await;

    // Missing required query param — should return 400
    let resp = client.get(format!("{}/unlock", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["message"].is_string(), "error response must have message field");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unlock_is_public_endpoint() {
    // No authentication — should still process (return 400 for invalid token, not 401)
    let client = test_client().await;

    let resp = client.get(format!("{}/unlock?token=no-auth-needed-test", V2_BASE)).dispatch().await;

    // Should return 400 (bad token), NOT 401 (unauthorized)
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unlock_empty_token_returns_400() {
    let client = test_client().await;

    // Empty token should be rejected
    let resp = client.get(format!("{}/unlock?token=", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}
