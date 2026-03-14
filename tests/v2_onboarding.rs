mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::Status;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /onboarding/status
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_status_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["status"].is_string());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_status_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /onboarding/complete
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_complete_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::NoContent);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_complete_already_completed() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Complete once
    client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;

    // Complete again
    let resp = client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;

    // Should either succeed idempotently (204) or return conflict (409)
    assert!(
        resp.status() == Status::NoContent || resp.status() == Status::Conflict,
        "expected 204 or 409, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_complete_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
