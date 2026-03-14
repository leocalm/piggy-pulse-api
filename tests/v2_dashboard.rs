mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::Status;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/current-period
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/current-period", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["spent"].is_number());
    assert!(body["target"].is_number());
    assert!(body["daysRemaining"].is_number());
    assert!(body["daysInPeriod"].is_number());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_with_period_id() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/dashboard/current-period", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/net-position
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_net_position_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/net-position", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["total"].is_number());
    assert!(body["numberOfAccounts"].is_number());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_net_position_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/dashboard/net-position", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/budget-stability
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_budget_stability_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/budget-stability", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["stability"].is_number());
    assert!(body["periodsWithinRange"].is_number());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_budget_stability_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/dashboard/budget-stability", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
