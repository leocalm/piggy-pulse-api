mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_period, create_target, create_transaction, create_transaction_with_vendor, create_vendor};
use common::{V2_BASE, test_client};
use rocket::http::Status;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /categories/{id}/detail
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_detail_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let cat_id = create_category(&client, "Groceries Detail", "expense").await;
    let account_id = create_account(&client, "Checking", 200_000).await;

    create_target(&client, &cat_id, 20_000).await;

    create_transaction(&client, &account_id, &cat_id, 5000, "2026-03-05").await;
    create_transaction(&client, &account_id, &cat_id, 8000, "2026-03-15").await;

    let resp = client
        .get(format!("{}/categories/{}/detail?periodId={}", V2_BASE, cat_id, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["id"], cat_id);
    assert_eq!(body["name"], "Groceries Detail");
    assert_eq!(body["periodSpend"], 13_000);
    assert_eq!(body["budgeted"], 20_000);
    assert!(body["trend"].is_array());
    assert!(body["recentTransactions"].is_array());
    assert_eq!(body["recentTransactions"].as_array().unwrap().len(), 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_detail_no_budget() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let cat_id = create_category(&client, "No Budget Cat", "expense").await;
    let account_id = create_account(&client, "Savings", 50_000).await;

    create_transaction(&client, &account_id, &cat_id, 3000, "2026-03-10").await;

    let resp = client
        .get(format!("{}/categories/{}/detail?periodId={}", V2_BASE, cat_id, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["periodSpend"], 3_000);
    assert!(body["budgeted"].is_null());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_detail_recent_transactions_include_vendor() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let cat_id = create_category(&client, "Food With Vendor", "expense").await;
    let account_id = create_account(&client, "Bank", 100_000).await;
    let vendor_id = create_vendor(&client, "Lidl").await;

    create_transaction_with_vendor(&client, &account_id, &cat_id, 2500, "2026-03-12", &vendor_id).await;

    let resp = client
        .get(format!("{}/categories/{}/detail?periodId={}", V2_BASE, cat_id, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let txns = body["recentTransactions"].as_array().unwrap();
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0]["vendorId"], vendor_id);
    assert_eq!(txns[0]["vendorName"], "Lidl");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_detail_stability_dots_within_budget() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create two periods and transactions
    let period1 = create_period(&client, "2026-01-01", "2026-01-31").await;
    let period2 = create_period(&client, "2026-02-01", "2026-02-28").await;
    let cat_id = create_category(&client, "Stability Cat", "expense").await;
    let account_id = create_account(&client, "Bank2", 100_000).await;
    create_target(&client, &cat_id, 10_000).await;

    // Period 1: spend 5000 (within budget of 10000)
    create_transaction(&client, &account_id, &cat_id, 5000, "2026-01-15").await;
    // Period 2: spend 12000 (over budget of 10000)
    create_transaction(&client, &account_id, &cat_id, 12_000, "2026-02-15").await;

    let resp = client
        .get(format!("{}/categories/{}/detail?periodId={}", V2_BASE, cat_id, period2))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let trend = body["trend"].as_array().unwrap();
    // Should have at least 2 trend items (one per closed period)
    assert!(trend.len() >= 2);

    // Find the trend item for period1 (spend 5000, within budget of 10000)
    let item1 = trend
        .iter()
        .find(|d| d["periodId"].as_str() == Some(&period1))
        .expect("period1 trend item should exist");
    assert_eq!(item1["totalSpend"], 5_000);

    // Find the trend item for period2 (spend 12000, over budget of 10000)
    let item2 = trend
        .iter()
        .find(|d| d["periodId"].as_str() == Some(&period2))
        .expect("period2 trend item should exist");
    assert_eq!(item2["totalSpend"], 12_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_detail_not_found_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!(
            "{}/categories/00000000-0000-0000-0000-000000000099/detail?periodId={}",
            V2_BASE, period_id
        ))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_detail_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "X Cat", "expense").await;

    let resp = client.get(format!("{}/categories/{}/detail", V2_BASE, cat_id)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_detail_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!(
            "{}/categories/00000000-0000-0000-0000-000000000001/detail?periodId=00000000-0000-0000-0000-000000000001",
            V2_BASE
        ))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /categories/{id}/trend
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_trend_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period1 = create_period(&client, "2026-01-01", "2026-01-31").await;
    let period2 = create_period(&client, "2026-02-01", "2026-02-28").await;
    let period3 = create_period(&client, "2026-03-01", "2026-03-31").await;
    let cat_id = create_category(&client, "Trend Cat", "expense").await;
    let account_id = create_account(&client, "Trend Bank", 500_000).await;

    create_transaction(&client, &account_id, &cat_id, 3000, "2026-01-10").await;
    create_transaction(&client, &account_id, &cat_id, 5000, "2026-02-10").await;
    create_transaction(&client, &account_id, &cat_id, 7000, "2026-03-10").await;

    let resp = client.get(format!("{}/categories/{}/trend", V2_BASE, cat_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let arr = body.as_array().expect("trend is array");
    // Should have 3 periods
    assert_eq!(arr.len(), 3);

    // Ordered newest first
    let period3_item = arr.iter().find(|i| i["periodId"].as_str() == Some(&period3)).expect("period3");
    assert_eq!(period3_item["totalSpend"], 7000);

    let period2_item = arr.iter().find(|i| i["periodId"].as_str() == Some(&period2)).expect("period2");
    assert_eq!(period2_item["totalSpend"], 5000);

    let period1_item = arr.iter().find(|i| i["periodId"].as_str() == Some(&period1)).expect("period1");
    assert_eq!(period1_item["totalSpend"], 3000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_trend_zero_spend_periods_included() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let _period1 = create_period(&client, "2026-01-01", "2026-01-31").await;
    let _period2 = create_period(&client, "2026-02-01", "2026-02-28").await;
    let cat_id = create_category(&client, "Empty Trend Cat", "expense").await;

    let resp = client.get(format!("{}/categories/{}/trend", V2_BASE, cat_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let arr = body.as_array().unwrap();
    // Both periods appear with zero spend
    assert_eq!(arr.len(), 2);
    for item in arr {
        assert_eq!(item["totalSpend"], 0);
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_trend_limit_respected() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    create_period(&client, "2026-01-01", "2026-01-31").await;
    create_period(&client, "2026-02-01", "2026-02-28").await;
    create_period(&client, "2026-03-01", "2026-03-31").await;
    let cat_id = create_category(&client, "Limit Trend Cat", "expense").await;

    let resp = client.get(format!("{}/categories/{}/trend?limit=2", V2_BASE, cat_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_trend_not_found_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .get(format!("{}/categories/00000000-0000-0000-0000-000000000099/trend", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_trend_invalid_uuid_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/categories/not-a-uuid/trend", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_trend_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/categories/00000000-0000-0000-0000-000000000001/trend", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
