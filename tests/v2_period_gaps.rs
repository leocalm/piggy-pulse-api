mod common;

use common::auth::create_user_and_login;
use common::entities::create_period;
use common::{V2_BASE, test_client};
use rocket::http::Status;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /periods/gaps
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_period_gaps_no_periods_returns_empty() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/periods/gaps", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_period_gaps_consecutive_periods_no_gaps() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Two back-to-back periods (Jan then Feb, no gap between them)
    create_period(&client, "2026-01-01", "2026-01-31").await;
    create_period(&client, "2026-02-01", "2026-02-28").await;

    let resp = client.get(format!("{}/periods/gaps", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0, "back-to-back periods should have no gap");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_period_gaps_one_gap_detected() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Jan 1–15, then Jan 21–31 → gap from Jan 16 to Jan 20
    create_period(&client, "2026-01-01", "2026-01-15").await;
    create_period(&client, "2026-01-21", "2026-01-31").await;

    let resp = client.get(format!("{}/periods/gaps", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let gaps = body.as_array().unwrap();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0]["startDate"], "2026-01-16");
    assert_eq!(gaps[0]["endDate"], "2026-01-20");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_period_gaps_multiple_gaps() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Three periods with gaps in between
    create_period(&client, "2026-01-01", "2026-01-10").await;
    create_period(&client, "2026-01-15", "2026-01-20").await;
    create_period(&client, "2026-01-25", "2026-01-31").await;

    let resp = client.get(format!("{}/periods/gaps", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let gaps = body.as_array().unwrap();
    assert_eq!(gaps.len(), 2);

    // First gap: Jan 11–14
    assert_eq!(gaps[0]["startDate"], "2026-01-11");
    assert_eq!(gaps[0]["endDate"], "2026-01-14");

    // Second gap: Jan 21–24
    assert_eq!(gaps[1]["startDate"], "2026-01-21");
    assert_eq!(gaps[1]["endDate"], "2026-01-24");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_period_gaps_single_period_no_gaps() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client.get(format!("{}/periods/gaps", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0, "single period has no internal gaps");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_period_gaps_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client.get(format!("{}/periods/gaps", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
