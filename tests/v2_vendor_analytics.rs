mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_period, create_transaction_with_vendor, create_vendor};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /vendors/stats
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_stats_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let cat_id = create_category(&client, "Food", "expense").await;
    let account_id = create_account(&client, "Checking", 100_000).await;
    let vendor_a = create_vendor(&client, "Supermarket A").await;
    let vendor_b = create_vendor(&client, "Gas Station B").await;

    create_transaction_with_vendor(&client, &account_id, &cat_id, 5000, "2026-03-10", &vendor_a).await;
    create_transaction_with_vendor(&client, &account_id, &cat_id, 3000, "2026-03-15", &vendor_b).await;

    let resp = client.get(format!("{}/vendors/stats?periodId={}", V2_BASE, period_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert!(body["totalVendors"].as_i64().unwrap() >= 2);
    assert_eq!(body["totalSpendThisPeriod"], 8000);
    // avgSpendPerVendor = 8000 / 2 vendors that spent = 4000
    assert_eq!(body["avgSpendPerVendor"], 4000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_stats_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/vendors/stats", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_stats_invalid_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/vendors/stats?periodId=not-a-uuid", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_stats_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/vendors/stats?periodId=00000000-0000-0000-0000-000000000001", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /vendors/{id}/detail
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_detail_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let cat_id = create_category(&client, "Groceries", "expense").await;
    let account_id = create_account(&client, "Main", 200_000).await;
    let vendor_id = create_vendor(&client, "Whole Foods").await;

    create_transaction_with_vendor(&client, &account_id, &cat_id, 4000, "2026-03-05", &vendor_id).await;
    create_transaction_with_vendor(&client, &account_id, &cat_id, 6000, "2026-03-20", &vendor_id).await;

    let resp = client
        .get(format!("{}/vendors/{}/detail?periodId={}", V2_BASE, vendor_id, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["id"], vendor_id);
    assert_eq!(body["name"], "Whole Foods");
    assert_eq!(body["periodSpend"], 10_000);
    assert_eq!(body["transactionCount"], 2);
    assert_eq!(body["averageTransactionAmount"], 5_000);
    assert!(body["trend"].is_array());
    assert!(body["topCategories"].is_array());
    assert_eq!(body["topCategories"][0]["categoryName"], "Groceries");
    assert!(body["recentTransactions"].is_array());
    assert_eq!(body["recentTransactions"].as_array().unwrap().len(), 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_detail_no_transactions_in_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let vendor_id = create_vendor(&client, "Empty Vendor").await;

    let resp = client
        .get(format!("{}/vendors/{}/detail?periodId={}", V2_BASE, vendor_id, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["periodSpend"], 0);
    assert_eq!(body["transactionCount"], 0);
    assert_eq!(body["averageTransactionAmount"], 0);
    assert_eq!(body["recentTransactions"].as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_detail_not_found_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!(
            "{}/vendors/00000000-0000-0000-0000-000000000099/detail?periodId={}",
            V2_BASE, period_id
        ))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_detail_invalid_uuid_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .get(format!("{}/vendors/not-a-uuid/detail?periodId=00000000-0000-0000-0000-000000000001", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_detail_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!(
            "{}/vendors/00000000-0000-0000-0000-000000000001/detail?periodId=00000000-0000-0000-0000-000000000001",
            V2_BASE
        ))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /vendors/{id}/merge
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_merge_vendor_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Misc", "expense").await;
    let account_id = create_account(&client, "Account", 100_000).await;
    let source_id = create_vendor(&client, "Old Vendor").await;
    let target_id = create_vendor(&client, "New Vendor").await;

    // Create a transaction on the source vendor
    create_transaction_with_vendor(&client, &account_id, &cat_id, 1000, "2026-03-01", &source_id).await;

    let payload = serde_json::json!({ "targetVendorId": target_id });

    let resp = client
        .post(format!("{}/vendors/{}/merge", V2_BASE, source_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NoContent);

    // Source vendor should be deleted
    let get_resp = client.get(format!("{}/vendors/{}", V2_BASE, source_id)).dispatch().await;
    // Should be 404 or the vendor list should not contain source anymore
    // Since we don't have a GET /vendors/{id} endpoint, check the list
    let list_resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let ids: Vec<&str> = list_body["data"].as_array().unwrap().iter().filter_map(|v| v["id"].as_str()).collect();
    assert!(!ids.contains(&source_id.as_str()), "Source vendor should be deleted after merge");
    assert!(ids.contains(&target_id.as_str()), "Target vendor should still exist");
    let _ = get_resp; // suppress unused warning
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_merge_vendor_same_source_and_target_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let vendor_id = create_vendor(&client, "Self Vendor").await;

    let payload = serde_json::json!({ "targetVendorId": vendor_id });

    let resp = client
        .post(format!("{}/vendors/{}/merge", V2_BASE, vendor_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_merge_vendor_source_not_found_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let target_id = create_vendor(&client, "Real Target").await;

    let payload = serde_json::json!({ "targetVendorId": target_id });

    let resp = client
        .post(format!("{}/vendors/00000000-0000-0000-0000-000000000099/merge", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_merge_vendor_unauthenticated_returns_401() {
    let client = test_client().await;

    let payload = serde_json::json!({ "targetVendorId": "00000000-0000-0000-0000-000000000001" });

    let resp = client
        .post(format!("{}/vendors/00000000-0000-0000-0000-000000000002/merge", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /vendors — verify totalSpend field is now present
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_vendors_includes_total_spend() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Food2", "expense").await;
    let account_id = create_account(&client, "Bank", 100_000).await;
    let vendor_id = create_vendor(&client, "Test Vendor TotalSpend").await;

    create_transaction_with_vendor(&client, &account_id, &cat_id, 2500, "2026-03-10", &vendor_id).await;
    create_transaction_with_vendor(&client, &account_id, &cat_id, 3500, "2026-03-11", &vendor_id).await;

    let resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let vendor_entry = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"].as_str() == Some(&vendor_id))
        .expect("vendor should be in list");

    assert_eq!(vendor_entry["totalSpend"], 6000);
    assert_eq!(vendor_entry["numberOfTransactions"], 2);
}
