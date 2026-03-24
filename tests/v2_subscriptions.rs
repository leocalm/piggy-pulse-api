mod common;

use common::auth::create_user_and_login;
use common::entities::{create_category, create_subscription, create_vendor};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /subscriptions
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_subscription_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Streaming", "expense").await;

    let payload = serde_json::json!({
        "name": "Netflix",
        "categoryId": cat_id,
        "vendorId": null,
        "billingAmount": 1499,
        "billingCycle": "monthly",
        "billingDay": 15,
        "nextChargeDate": "2026-04-15"
    });

    let resp = client
        .post(format!("{}/subscriptions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["name"], "Netflix");
    assert_eq!(body["billingAmount"], 1499);
    assert_eq!(body["billingCycle"], "monthly");
    assert_eq!(body["billingDay"], 15);
    assert_eq!(body["nextChargeDate"], "2026-04-15");
    assert_eq!(body["status"], "active");
    assert!(body["cancelledAt"].is_null());
    assert!(body["id"].is_string());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_subscription_with_vendor() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Software", "expense").await;
    let vendor_id = create_vendor(&client, "Spotify").await;

    let payload = serde_json::json!({
        "name": "Spotify Premium",
        "categoryId": cat_id,
        "vendorId": vendor_id,
        "billingAmount": 999,
        "billingCycle": "monthly",
        "billingDay": 1,
        "nextChargeDate": "2026-04-01"
    });

    let resp = client
        .post(format!("{}/subscriptions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["vendorId"], vendor_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_subscription_invalid_amount_returns_422() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Misc", "expense").await;

    let payload = serde_json::json!({
        "name": "Zero Cost",
        "categoryId": cat_id,
        "vendorId": null,
        "billingAmount": 0,
        "billingCycle": "monthly",
        "billingDay": 1,
        "nextChargeDate": "2026-04-01"
    });

    let resp = client
        .post(format!("{}/subscriptions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_subscription_unauthenticated_returns_401() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "name": "X",
        "categoryId": "00000000-0000-0000-0000-000000000001",
        "vendorId": null,
        "billingAmount": 1000,
        "billingCycle": "monthly",
        "billingDay": 1,
        "nextChargeDate": "2026-04-01"
    });

    let resp = client
        .post(format!("{}/subscriptions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /subscriptions
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_subscriptions_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Streaming", "expense").await;
    create_subscription(&client, "Netflix", &cat_id, 1499, "monthly", "2026-04-15").await;
    create_subscription(&client, "Spotify", &cat_id, 999, "monthly", "2026-04-01").await;

    let resp = client.get(format!("{}/subscriptions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let arr = body.as_array().expect("list response is array");
    assert_eq!(arr.len(), 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_subscriptions_status_filter() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Software", "expense").await;
    let sub_id = create_subscription(&client, "Adobe CC", &cat_id, 5999, "monthly", "2026-04-01").await;

    // Cancel one subscription
    client.post(format!("{}/subscriptions/{}/cancel", V2_BASE, sub_id)).dispatch().await;

    create_subscription(&client, "GitHub Pro", &cat_id, 400, "monthly", "2026-04-05").await;

    // Filter by active
    let resp = client.get(format!("{}/subscriptions?status=active", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let active: Vec<&Value> = body.as_array().unwrap().iter().filter(|s| s["status"] == "active").collect();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0]["name"], "GitHub Pro");

    // Filter by cancelled
    let resp = client.get(format!("{}/subscriptions?status=cancelled", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let cancelled: Vec<&Value> = body.as_array().unwrap().iter().filter(|s| s["status"] == "cancelled").collect();
    assert_eq!(cancelled.len(), 1);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_subscriptions_invalid_status_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/subscriptions?status=bogus", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_subscriptions_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client.get(format!("{}/subscriptions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /subscriptions/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_subscription_detail_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Fitness", "expense").await;
    let sub_id = create_subscription(&client, "Gym Membership", &cat_id, 3000, "monthly", "2026-04-01").await;

    let resp = client.get(format!("{}/subscriptions/{}", V2_BASE, sub_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["id"], sub_id);
    assert_eq!(body["name"], "Gym Membership");
    assert_eq!(body["billingAmount"], 3000);
    assert_eq!(body["status"], "active");
    assert!(body["billingHistory"].is_array());
    assert_eq!(body["billingHistory"].as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_subscription_detail_not_found_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .get(format!("{}/subscriptions/00000000-0000-0000-0000-000000000099", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_subscription_detail_invalid_uuid_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/subscriptions/not-a-uuid", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_subscription_detail_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/subscriptions/00000000-0000-0000-0000-000000000001", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /subscriptions/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_subscription_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Cloud", "expense").await;
    let sub_id = create_subscription(&client, "iCloud 50GB", &cat_id, 99, "monthly", "2026-04-01").await;

    let update = serde_json::json!({
        "name": "iCloud 200GB",
        "categoryId": cat_id,
        "vendorId": null,
        "billingAmount": 299,
        "billingCycle": "monthly",
        "billingDay": 1,
        "nextChargeDate": "2026-04-01"
    });

    let resp = client
        .put(format!("{}/subscriptions/{}", V2_BASE, sub_id))
        .header(ContentType::JSON)
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["name"], "iCloud 200GB");
    assert_eq!(body["billingAmount"], 299);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_subscription_not_found_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "X", "expense").await;

    let update = serde_json::json!({
        "name": "Ghost",
        "categoryId": cat_id,
        "vendorId": null,
        "billingAmount": 500,
        "billingCycle": "yearly",
        "billingDay": 1,
        "nextChargeDate": "2027-01-01"
    });

    let resp = client
        .put(format!("{}/subscriptions/00000000-0000-0000-0000-000000000099", V2_BASE))
        .header(ContentType::JSON)
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /subscriptions/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_subscription_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Unused", "expense").await;
    let sub_id = create_subscription(&client, "Temp Sub", &cat_id, 100, "monthly", "2026-04-01").await;

    let resp = client.delete(format!("{}/subscriptions/{}", V2_BASE, sub_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NoContent);

    // Confirm it's gone
    let get_resp = client.get(format!("{}/subscriptions/{}", V2_BASE, sub_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_subscription_not_found_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .delete(format!("{}/subscriptions/00000000-0000-0000-0000-000000000099", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_subscription_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .delete(format!("{}/subscriptions/00000000-0000-0000-0000-000000000001", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /subscriptions/{id}/cancel
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_cancel_subscription_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "News", "expense").await;
    let sub_id = create_subscription(&client, "NYT Digital", &cat_id, 1700, "monthly", "2026-04-01").await;

    let resp = client.post(format!("{}/subscriptions/{}/cancel", V2_BASE, sub_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["status"], "cancelled");
    assert!(!body["cancelledAt"].is_null());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_cancel_subscription_already_cancelled_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Misc2", "expense").await;
    let sub_id = create_subscription(&client, "Already Gone", &cat_id, 200, "monthly", "2026-04-01").await;

    // Cancel once
    client.post(format!("{}/subscriptions/{}/cancel", V2_BASE, sub_id)).dispatch().await;

    // Cancel again — should be 404
    let resp = client.post(format!("{}/subscriptions/{}/cancel", V2_BASE, sub_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_cancel_subscription_not_found_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .post(format!("{}/subscriptions/00000000-0000-0000-0000-000000000099/cancel", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_cancel_subscription_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .post(format!("{}/subscriptions/00000000-0000-0000-0000-000000000001/cancel", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /subscriptions/upcoming
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_upcoming_charges_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Subscriptions", "expense").await;
    create_subscription(&client, "Service A", &cat_id, 500, "monthly", "2026-04-10").await;
    create_subscription(&client, "Service B", &cat_id, 800, "monthly", "2026-04-05").await;
    create_subscription(&client, "Service C", &cat_id, 1200, "yearly", "2026-04-20").await;

    let resp = client.get(format!("{}/subscriptions/upcoming", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let arr = body.as_array().expect("upcoming is array");
    // All 3 active subscriptions should appear, ordered by next_charge_date ASC
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["nextChargeDate"], "2026-04-05");
    assert_eq!(arr[1]["nextChargeDate"], "2026-04-10");
    assert_eq!(arr[2]["nextChargeDate"], "2026-04-20");

    // Check shape of each item
    for item in arr {
        assert!(item["subscriptionId"].is_string());
        assert!(item["name"].is_string());
        assert!(item["billingAmount"].is_number());
        assert!(item["billingCycle"].is_string());
        assert!(item["nextChargeDate"].is_string());
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_upcoming_charges_excludes_cancelled() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Cancelled Cat", "expense").await;
    let sub_id = create_subscription(&client, "Cancelled Sub", &cat_id, 999, "monthly", "2026-04-01").await;

    // Cancel it
    client.post(format!("{}/subscriptions/{}/cancel", V2_BASE, sub_id)).dispatch().await;

    let resp = client.get(format!("{}/subscriptions/upcoming", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 0, "cancelled subscriptions should not appear in upcoming");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_upcoming_charges_limit_clamped() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = create_category(&client, "Limit Test", "expense").await;
    for i in 0..5 {
        create_subscription(&client, &format!("Sub {}", i), &cat_id, 100 + i * 10, "monthly", "2026-04-01").await;
    }

    // Request limit=2
    let resp = client.get(format!("{}/subscriptions/upcoming?limit=2", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_upcoming_charges_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client.get(format!("{}/subscriptions/upcoming", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
