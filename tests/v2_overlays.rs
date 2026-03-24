mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// Helper: create overlay with full control
// ═══════════════════════════════════════════════════════════════════════════════

struct OverlayOpts<'a> {
    name: &'a str,
    start: &'a str,
    end: &'a str,
    mode: &'a str,
    total_cap: Option<i64>,
    category_caps: Option<Value>,
    rules: Option<Value>,
}

async fn create_overlay_full(client: &rocket::local::asynchronous::Client, opts: OverlayOpts<'_>) -> Value {
    let payload = serde_json::json!({
        "name": opts.name,
        "icon": null,
        "startDate": opts.start,
        "endDate": opts.end,
        "inclusionMode": opts.mode,
        "totalCapAmount": opts.total_cap,
        "categoryCaps": opts.category_caps.unwrap_or(serde_json::json!([])),
        "rules": opts.rules,
    });

    let resp = client
        .post(format!("{}/overlays", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_overlay_full failed");
    serde_json::from_str(&resp.into_string().await.unwrap()).unwrap()
}

/// Shorthand for creating a simple overlay with just name, dates, and mode
async fn create_simple_overlay(client: &rocket::local::asynchronous::Client, name: &str, start: &str, end: &str, mode: &str) -> Value {
    create_overlay_full(
        client,
        OverlayOpts {
            name,
            start,
            end,
            mode,
            total_cap: None,
            category_caps: None,
            rules: None,
        },
    )
    .await
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /overlays (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let body = create_overlay_full(
        &client,
        OverlayOpts {
            name: "Holiday Spending",
            start: "2026-04-01",
            end: "2026-04-30",
            mode: "all",
            total_cap: Some(100000),
            category_caps: None,
            rules: None,
        },
    )
    .await;

    // Assert ALL scalar fields
    common::assertions::assert_uuid(&body["id"]);
    assert_eq!(body["name"], "Holiday Spending");
    assert!(body["icon"].is_null());
    assert_eq!(body["startDate"], "2026-04-01");
    assert_eq!(body["endDate"], "2026-04-30");
    assert_eq!(body["inclusionMode"], "all");
    assert_eq!(body["totalCapAmount"], 100000);
    assert_eq!(body["spentAmount"], 0);
    assert_eq!(body["transactionCount"], 0);
    assert!(body["categoryCaps"].is_array());
    assert_eq!(body["categoryCaps"].as_array().unwrap().len(), 0);
    // rules should be present (empty)
    assert!(body["rules"].is_object() || body["rules"].is_null());
    // categoryBreakdown must be present as an array (empty when no transactions)
    assert!(body["categoryBreakdown"].is_array(), "expected categoryBreakdown array");
    assert_eq!(body["categoryBreakdown"].as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_with_total_cap_amount() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cap_value = 50000;
    let body = create_overlay_full(
        &client,
        OverlayOpts {
            name: "Capped Overlay",
            start: "2026-05-01",
            end: "2026-05-31",
            mode: "manual",
            total_cap: Some(cap_value),
            category_caps: None,
            rules: None,
        },
    )
    .await;

    assert_eq!(body["totalCapAmount"], cap_value);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_with_category_caps() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Food", "expense").await;

    let cap_amount = 25000;
    let caps = serde_json::json!([{
        "categoryId": cat_id,
        "capAmount": cap_amount
    }]);

    let body = create_overlay_full(
        &client,
        OverlayOpts {
            name: "With Caps",
            start: "2026-06-01",
            end: "2026-06-30",
            mode: "rules",
            total_cap: None,
            category_caps: Some(caps),
            rules: Some(serde_json::json!({"categoryIds": [cat_id]})),
        },
    )
    .await;

    let cat_caps = body["categoryCaps"].as_array().unwrap();
    assert_eq!(cat_caps.len(), 1);
    assert_eq!(cat_caps[0]["categoryId"], cat_id);
    assert_eq!(cat_caps[0]["capAmount"], cap_amount);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_missing_required_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Missing name (empty string fails validation)
    let payload = serde_json::json!({
        "name": "",
        "startDate": "2026-03-01",
        "endDate": "2026-03-15",
        "inclusionMode": "manual"
    });

    let resp = client
        .post(format!("{}/overlays", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_end_date_before_start_date() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "name": "Bad Dates",
        "startDate": "2026-06-30",
        "endDate": "2026-06-01",
        "inclusionMode": "manual"
    });

    let resp = client
        .post(format!("{}/overlays", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "name": "No Auth",
        "startDate": "2026-03-01",
        "endDate": "2026-03-15",
        "inclusionMode": "manual"
    });

    let resp = client
        .post(format!("{}/overlays", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /overlays (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_overlays_two_appear() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let o1 = create_simple_overlay(&client, "First", "2026-07-01", "2026-07-15", "manual").await;
    let o2 = create_simple_overlay(&client, "Second", "2026-07-16", "2026-07-31", "all").await;

    let resp = client.get(format!("{}/overlays", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);

    let data = body["data"].as_array().unwrap();
    assert!(data.len() >= 2);

    let ids: Vec<&str> = data.iter().map(|o| o["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&o1["id"].as_str().unwrap()));
    assert!(ids.contains(&o2["id"].as_str().unwrap()));
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_overlays_pagination() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    create_simple_overlay(&client, "P1", "2026-08-01", "2026-08-10", "manual").await;
    create_simple_overlay(&client, "P2", "2026-08-11", "2026-08-20", "manual").await;
    create_simple_overlay(&client, "P3", "2026-08-21", "2026-08-31", "manual").await;

    let resp = client.get(format!("{}/overlays?limit=1", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["hasMore"], true);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_overlays_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/overlays", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /overlays/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_overlay_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let created = create_overlay_full(
        &client,
        OverlayOpts {
            name: "Get Me",
            start: "2026-09-01",
            end: "2026-09-30",
            mode: "all",
            total_cap: Some(75000),
            category_caps: None,
            rules: None,
        },
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let resp = client.get(format!("{}/overlays/{}", V2_BASE, id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["id"], id);
    assert_eq!(body["name"], "Get Me");
    assert_eq!(body["startDate"], "2026-09-01");
    assert_eq!(body["endDate"], "2026-09-30");
    assert_eq!(body["inclusionMode"], "all");
    assert_eq!(body["totalCapAmount"], 75000);
    assert_eq!(body["spentAmount"], 0);
    assert_eq!(body["transactionCount"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_overlay_computed_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create overlay with mode "all" to auto-include transactions in date range
    let overlay = create_simple_overlay(&client, "Computed", "2026-10-01", "2026-10-31", "all").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    // Create supporting entities
    let account_id = common::entities::create_account(&client, "Compute Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "Compute Cat", "expense").await;

    // Create transactions in overlay date range
    let amount1 = 5000;
    let amount2 = 7000;
    common::entities::create_transaction(&client, &account_id, &category_id, amount1, "2026-10-05").await;
    common::entities::create_transaction(&client, &account_id, &category_id, amount2, "2026-10-15").await;

    // GET the overlay — spentAmount, transactionCount, and categoryBreakdown must reflect real data
    let resp = client.get(format!("{}/overlays/{}", V2_BASE, overlay_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["spentAmount"], amount1 + amount2); // 12000
    assert_eq!(body["transactionCount"], 2);

    // categoryBreakdown must be populated with the single category's total
    let breakdown = body["categoryBreakdown"].as_array().expect("categoryBreakdown must be array");
    assert_eq!(breakdown.len(), 1, "expected 1 category in breakdown");
    assert_eq!(breakdown[0]["categoryId"], category_id);
    assert_eq!(breakdown[0]["categoryName"], "Compute Cat");
    assert_eq!(breakdown[0]["amount"], amount1 + amount2); // sorted descending by amount
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_overlay_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/overlays/{}", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_overlay_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/overlays/{}", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /overlays/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_overlay_name_persists() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let created = create_overlay_full(
        &client,
        OverlayOpts {
            name: "Before",
            start: "2026-11-01",
            end: "2026-11-30",
            mode: "manual",
            total_cap: Some(100000),
            category_caps: None,
            rules: None,
        },
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let update_payload = serde_json::json!({
        "name": "After Update",
        "icon": null,
        "startDate": "2026-11-01",
        "endDate": "2026-11-30",
        "inclusionMode": "manual",
        "totalCapAmount": 200000,
        "categoryCaps": [],
        "rules": null
    });

    let resp = client
        .put(format!("{}/overlays/{}", V2_BASE, id))
        .header(ContentType::JSON)
        .body(update_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Verify persistence via GET
    let resp = client.get(format!("{}/overlays/{}", V2_BASE, id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "After Update");
    assert_eq!(body["totalCapAmount"], 200000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_overlay_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "name": "Ghost",
        "startDate": "2026-06-01",
        "endDate": "2026-06-30",
        "inclusionMode": "manual"
    });

    let resp = client
        .put(format!("{}/overlays/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_overlay_no_auth() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/overlays/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "name": "X",
                "startDate": "2026-01-01",
                "endDate": "2026-01-31",
                "inclusionMode": "manual"
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /overlays/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_overlay_then_get_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let created = create_simple_overlay(&client, "To Delete", "2026-12-01", "2026-12-31", "manual").await;
    let id = created["id"].as_str().unwrap();

    let resp = client.delete(format!("{}/overlays/{}", V2_BASE, id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify it's gone
    let resp = client.get(format!("{}/overlays/{}", V2_BASE, id)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_overlay_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/overlays/{}", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_overlay_no_auth() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/overlays/{}", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /overlays/{id}/transactions
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overlay_transactions_all_mode_shows_included() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "All Txs", "2027-01-01", "2027-01-31", "all").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let account_id = common::entities::create_account(&client, "Tx Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "Tx Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 3000, "2027-01-15").await;

    let resp = client.get(format!("{}/overlays/{}/transactions", V2_BASE, overlay_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let data = body["data"].as_array().unwrap();

    // Find the transaction we created
    let found = data.iter().find(|t| t["id"].as_str().unwrap() == tx_id);
    assert!(found.is_some(), "Transaction should appear in overlay transactions");

    let tx = found.unwrap();
    assert_eq!(tx["membership"]["isIncluded"], true);
    assert_eq!(tx["membership"]["inclusionSource"], "all");
    assert_eq!(tx["amount"], 3000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overlay_transactions_outside_range_excluded() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "Range Check", "2027-02-01", "2027-02-15", "all").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let account_id = common::entities::create_account(&client, "Range Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "Range Cat", "expense").await;

    // In range
    let in_range_id = common::entities::create_transaction(&client, &account_id, &category_id, 1000, "2027-02-05").await;
    // Out of range
    let _out_id = common::entities::create_transaction(&client, &account_id, &category_id, 2000, "2027-02-20").await;

    let resp = client.get(format!("{}/overlays/{}/transactions", V2_BASE, overlay_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let data = body["data"].as_array().unwrap();
    let ids: Vec<&str> = data.iter().map(|t| t["id"].as_str().unwrap()).collect();

    assert!(ids.contains(&in_range_id.as_str()), "In-range transaction should appear");
    // Out-of-range transaction should not appear
    assert!(!ids.contains(&_out_id.as_str()), "Out-of-range transaction should not appear");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overlay_transactions_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/overlays/{}/transactions", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overlay_transactions_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/overlays/{}/transactions", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /overlays/{id}/transactions/{txId}/include
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_include_transaction_manual_mode() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "Manual Include", "2027-03-01", "2027-03-31", "manual").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let account_id = common::entities::create_account(&client, "Inc Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "Inc Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 4000, "2027-03-10").await;

    // Include the transaction
    let resp = client
        .post(format!("{}/overlays/{}/transactions/{}/include", V2_BASE, overlay_id, tx_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify membership via GET transactions
    let resp = client.get(format!("{}/overlays/{}/transactions", V2_BASE, overlay_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let data = body["data"].as_array().unwrap();
    let found = data.iter().find(|t| t["id"].as_str().unwrap() == tx_id);
    assert!(found.is_some());

    let tx = found.unwrap();
    assert_eq!(tx["membership"]["isIncluded"], true);
    assert_eq!(tx["membership"]["inclusionSource"], "manual");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_include_transaction_already_included_409() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "Double Include", "2027-04-01", "2027-04-30", "manual").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let account_id = common::entities::create_account(&client, "409 Inc Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "409 Inc Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 1000, "2027-04-10").await;

    // First include — should succeed
    let resp = client
        .post(format!("{}/overlays/{}/transactions/{}/include", V2_BASE, overlay_id, tx_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Second include — should 409
    let resp = client
        .post(format!("{}/overlays/{}/transactions/{}/include", V2_BASE, overlay_id, tx_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_include_transaction_overlay_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = common::entities::create_account(&client, "IncNF Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "IncNF Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 1000, "2027-05-10").await;

    let resp = client
        .post(format!("{}/overlays/{}/transactions/{}/include", V2_BASE, Uuid::new_v4(), tx_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_include_transaction_tx_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "Inc TxNF", "2027-05-01", "2027-05-31", "manual").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let resp = client
        .post(format!("{}/overlays/{}/transactions/{}/include", V2_BASE, overlay_id, Uuid::new_v4()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_include_transaction_no_auth() {
    let client = test_client().await;

    let resp = client
        .post(format!("{}/overlays/{}/transactions/{}/include", V2_BASE, Uuid::new_v4(), Uuid::new_v4()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /overlays/{id}/transactions/{txId}/exclude
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_transaction_all_mode() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "Exclude All", "2027-06-01", "2027-06-30", "all").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let account_id = common::entities::create_account(&client, "Excl Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "Excl Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 6000, "2027-06-15").await;

    // Exclude the transaction
    let resp = client
        .delete(format!("{}/overlays/{}/transactions/{}/exclude", V2_BASE, overlay_id, tx_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify membership via GET transactions
    let resp = client.get(format!("{}/overlays/{}/transactions", V2_BASE, overlay_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let data = body["data"].as_array().unwrap();
    let found = data.iter().find(|t| t["id"].as_str().unwrap() == tx_id);
    assert!(found.is_some());

    let tx = found.unwrap();
    assert_eq!(tx["membership"]["isIncluded"], false);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_transaction_already_excluded_409() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "Double Exclude", "2027-07-01", "2027-07-31", "all").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let account_id = common::entities::create_account(&client, "409 Excl Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "409 Excl Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 2000, "2027-07-10").await;

    // First exclude — should succeed
    let resp = client
        .delete(format!("{}/overlays/{}/transactions/{}/exclude", V2_BASE, overlay_id, tx_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Second exclude — should 409
    let resp = client
        .delete(format!("{}/overlays/{}/transactions/{}/exclude", V2_BASE, overlay_id, tx_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_transaction_overlay_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = common::entities::create_account(&client, "ExclNF Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "ExclNF Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 1000, "2027-08-10").await;

    let resp = client
        .delete(format!("{}/overlays/{}/transactions/{}/exclude", V2_BASE, Uuid::new_v4(), tx_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_transaction_no_auth() {
    let client = test_client().await;

    let resp = client
        .delete(format!("{}/overlays/{}/transactions/{}/exclude", V2_BASE, Uuid::new_v4(), Uuid::new_v4()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Computed fields — spentAmount and transactionCount
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_computed_fields_three_transactions() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "Computed 3", "2027-09-01", "2027-09-30", "all").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let account_id = common::entities::create_account(&client, "Comp Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "Comp Cat", "expense").await;

    let a1 = 5000;
    let a2 = 3000;
    let a3 = 7000;
    common::entities::create_transaction(&client, &account_id, &category_id, a1, "2027-09-05").await;
    common::entities::create_transaction(&client, &account_id, &category_id, a2, "2027-09-15").await;
    common::entities::create_transaction(&client, &account_id, &category_id, a3, "2027-09-25").await;

    let resp = client.get(format!("{}/overlays/{}", V2_BASE, overlay_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["spentAmount"], a1 + a2 + a3); // 15000
    assert_eq!(body["transactionCount"], 3);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_computed_fields_exclude_updates_counts() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let overlay = create_simple_overlay(&client, "Comp Excl", "2027-10-01", "2027-10-31", "all").await;
    let overlay_id = overlay["id"].as_str().unwrap();

    let account_id = common::entities::create_account(&client, "CompExcl Acc", 500000).await;
    let category_id = common::entities::create_category(&client, "CompExcl Cat", "expense").await;

    let a1 = 5000;
    let a2 = 3000;
    let a3 = 7000;
    let tx1_id = common::entities::create_transaction(&client, &account_id, &category_id, a1, "2027-10-05").await;
    common::entities::create_transaction(&client, &account_id, &category_id, a2, "2027-10-15").await;
    common::entities::create_transaction(&client, &account_id, &category_id, a3, "2027-10-25").await;

    // Verify initial counts
    let resp = client.get(format!("{}/overlays/{}", V2_BASE, overlay_id)).dispatch().await;
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["spentAmount"], a1 + a2 + a3); // 15000
    assert_eq!(body["transactionCount"], 3);

    // Exclude one transaction
    let resp = client
        .delete(format!("{}/overlays/{}/transactions/{}/exclude", V2_BASE, overlay_id, tx1_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify updated counts
    let resp = client.get(format!("{}/overlays/{}", V2_BASE, overlay_id)).dispatch().await;
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["spentAmount"], a2 + a3); // 10000
    assert_eq!(body["transactionCount"], 2);
}
