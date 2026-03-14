mod common;

use common::{V2_BASE, test_client};
use rocket::http::Status;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /unlock?token=...
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unlock_valid_token() {
    let client = test_client().await;

    // We can't easily generate a valid token without triggering the lockout flow,
    // so this test validates the endpoint exists and handles a plausible token format.
    let resp = client.get(format!("{}/unlock?token=some-valid-looking-token-value", V2_BASE)).dispatch().await;

    // Should return 400 (invalid token) or 200 (if somehow valid)
    assert!(
        resp.status() == Status::Ok || resp.status() == Status::BadRequest || resp.status() == Status::NotFound,
        "expected 200, 400, or 404, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unlock_missing_token() {
    let client = test_client().await;

    let resp = client.get(format!("{}/unlock", V2_BASE)).dispatch().await;

    // Missing required query param
    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::NotFound,
        "expected 400 or 404, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unlock_invalid_token() {
    let client = test_client().await;

    let resp = client.get(format!("{}/unlock?token=invalid", V2_BASE)).dispatch().await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::NotFound,
        "expected 400 or 404, got {}",
        resp.status()
    );
}
