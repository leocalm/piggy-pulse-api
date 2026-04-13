mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /vendors (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_vendor_with_name_and_description() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .post(format!("{}/vendors", V2_BASE))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "name": "Supermarket",
                "description": "Weekly groceries"
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Assert ALL scalar fields
    assert_eq!(body["name"], "Supermarket");
    assert_eq!(body["description"], "Weekly groceries");
    assert_eq!(body["status"], "active");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_vendor_without_description() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .post(format!("{}/vendors", V2_BASE))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "name": "Gas Station",
                "description": null
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["name"], "Gas Station");
    assert!(body["description"].is_null(), "description should be null");
    assert_eq!(body["status"], "active");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_vendor_name_too_short() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .post(format!("{}/vendors", V2_BASE))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "name": "AB",
                "description": null
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_vendor_unauthenticated() {
    let client = test_client().await;

    let resp = client
        .post(format!("{}/vendors", V2_BASE))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "name": "No Auth Vendor",
                "description": null
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /vendors (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_vendors_both_appear() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let name_a = format!("VendorListA-{}", Uuid::new_v4());
    let name_b = format!("VendorListB-{}", Uuid::new_v4());
    common::entities::create_vendor(&client, &name_a).await;
    common::entities::create_vendor(&client, &name_b).await;

    let resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);

    let data = body["data"].as_array().unwrap();
    let names: Vec<&str> = data.iter().map(|v| v["name"].as_str().unwrap()).collect();
    assert!(names.contains(&name_a.as_str()), "Expected vendor A in list");
    assert!(names.contains(&name_b.as_str()), "Expected vendor B in list");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_vendors_number_of_transactions_zero() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let vendor_name = format!("ZeroTxVendor-{}", Uuid::new_v4());
    common::entities::create_vendor(&client, &vendor_name).await;

    let resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let data = body["data"].as_array().unwrap();
    let vendor = data.iter().find(|v| v["name"].as_str().unwrap() == vendor_name).expect("vendor in list");
    assert_eq!(vendor["numberOfTransactions"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_vendors_number_of_transactions_after_transaction() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Arrange: create vendor, account, category, period, transaction
    let vendor_name = format!("TxCountVendor-{}", Uuid::new_v4());
    let vendor_id = common::entities::create_vendor(&client, &vendor_name).await;
    let account_id = common::entities::create_account(&client, "TxCountAcct", 100_000).await;
    let category_id = common::entities::create_category(&client, "TxCountCat", "expense").await;
    let _period_id = common::entities::create_period(&client, "2026-01-01", "2026-12-31").await;

    // Create one transaction linked to the vendor
    common::entities::create_transaction_with_vendor(&client, &account_id, &category_id, 5_000, "2026-06-15", &vendor_id).await;

    let resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let data = body["data"].as_array().unwrap();
    let vendor = data.iter().find(|v| v["name"].as_str().unwrap() == vendor_name).expect("vendor in list");
    // Derived from arrangement: exactly 1 transaction was created for this vendor
    assert_eq!(vendor["numberOfTransactions"], 1);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_vendors_pagination() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    for i in 0..3 {
        common::entities::create_vendor(&client, &format!("PageVendor-{}-{}", i, Uuid::new_v4())).await;
    }

    let resp = client.get(format!("{}/vendors?limit=1", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["hasMore"], true);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_vendors_unauthenticated() {
    let client = test_client().await;

    let resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /vendors/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_vendor_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let vendor_id = common::entities::create_vendor(&client, "Before Update").await;

    let resp = client
        .put(format!("{}/vendors/{}", V2_BASE, vendor_id))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "name": "After Update",
                "description": "New description"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Verify persistence via subsequent GET list
    let list_resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let data = body["data"].as_array().unwrap();
    let vendor = data
        .iter()
        .find(|v| v["id"].as_str().unwrap() == vendor_id)
        .expect("vendor in list after update");
    assert_eq!(vendor["name"], "After Update");
    assert_eq!(vendor["description"], "New description");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_vendor_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .put(format!("{}/vendors/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "name": "Ghost Vendor",
                "description": null
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_vendor_unauthenticated() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/vendors/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "name": "NoAuth",
                "description": null
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /vendors/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_vendor_then_gone_from_list() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let vendor_name = format!("ToDelete-{}", Uuid::new_v4());
    let vendor_id = common::entities::create_vendor(&client, &vendor_name).await;

    let resp = client.delete(format!("{}/vendors/{}", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify gone via list
    let list_resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let data = body["data"].as_array().unwrap();
    let found = data.iter().any(|v| v["id"].as_str().unwrap() == vendor_id);
    assert!(!found, "Deleted vendor should not appear in list");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_vendor_with_transactions_is_blocked() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Set up: account, expense category, vendor, period, transaction
    let account_id = common::entities::create_account(&client, "Main", 100_000).await;
    let category_id = common::entities::create_category(&client, "Groceries", "expense").await;
    let vendor_id = common::entities::create_vendor(&client, "Albert Heijn").await;
    let _ = common::entities::create_period(&client, "2026-04-01", "2026-04-30").await;
    let _txn_id = common::entities::create_transaction_with_vendor(&client, &account_id, &category_id, 2500, "2026-04-15", &vendor_id).await;

    // Attempt to delete the vendor — should be rejected
    let resp = client.delete(format!("{}/vendors/{}", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let message = body["message"].as_str().unwrap_or("");
    assert!(
        message.contains("Cannot delete vendor with existing transactions"),
        "expected archive-instead error, got: {}",
        message
    );

    // Vendor must still exist in the list
    let list_resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let data = list_body["data"].as_array().unwrap();
    let found = data.iter().any(|v| v["id"].as_str().unwrap() == vendor_id);
    assert!(found, "Vendor should still exist after blocked delete");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_vendor_after_archive_when_used_is_still_blocked() {
    // Archiving a vendor does NOT permit hard delete — the transactions still
    // reference it. This locks in current behavior.
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = common::entities::create_account(&client, "Main", 100_000).await;
    let category_id = common::entities::create_category(&client, "Groceries", "expense").await;
    let vendor_id = common::entities::create_vendor(&client, "Lidl").await;
    let _ = common::entities::create_period(&client, "2026-04-01", "2026-04-30").await;
    let _txn_id = common::entities::create_transaction_with_vendor(&client, &account_id, &category_id, 1500, "2026-04-10", &vendor_id).await;

    // Archive the vendor first
    let archive_resp = client.post(format!("{}/vendors/{}/archive", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(archive_resp.status(), Status::Ok);

    // Hard delete should still be blocked
    let resp = client.delete(format!("{}/vendors/{}", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_vendor_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/vendors/{}", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_vendor_unauthenticated() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/vendors/{}", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /vendors/{id}/archive & /unarchive
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_vendor_status_inactive() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let vendor_name = format!("ArchiveMe-{}", Uuid::new_v4());
    let vendor_id = common::entities::create_vendor(&client, &vendor_name).await;

    let resp = client.post(format!("{}/vendors/{}/archive", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "inactive");
    assert_eq!(body["name"], vendor_name);
    common::assertions::assert_uuid(&body["id"]);

    // Verify via list
    let list_resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let data = list_body["data"].as_array().unwrap();
    let vendor = data.iter().find(|v| v["id"].as_str().unwrap() == vendor_id).expect("archived vendor in list");
    assert_eq!(vendor["status"], "inactive");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_already_archived_conflict() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let vendor_id = common::entities::create_vendor(&client, "DoubleArchive").await;

    // Archive first time
    let resp = client.post(format!("{}/vendors/{}/archive", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    // Archive second time — conflict
    let resp = client.post(format!("{}/vendors/{}/archive", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_vendor_status_active() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let vendor_name = format!("UnarchiveMe-{}", Uuid::new_v4());
    let vendor_id = common::entities::create_vendor(&client, &vendor_name).await;

    // Archive first
    client.post(format!("{}/vendors/{}/archive", V2_BASE, vendor_id)).dispatch().await;

    // Unarchive
    let resp = client.post(format!("{}/vendors/{}/unarchive", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "active");
    assert_eq!(body["name"], vendor_name);

    // Verify via list
    let list_resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let data = list_body["data"].as_array().unwrap();
    let vendor = data.iter().find(|v| v["id"].as_str().unwrap() == vendor_id).expect("unarchived vendor in list");
    assert_eq!(vendor["status"], "active");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_already_active_conflict() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let vendor_id = common::entities::create_vendor(&client, "AlreadyActive").await;

    // Vendor is already active — unarchive should conflict
    let resp = client.post(format!("{}/vendors/{}/unarchive", V2_BASE, vendor_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_vendor_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/vendors/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_vendor_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/vendors/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_vendor_unauthenticated() {
    let client = test_client().await;

    let resp = client.post(format!("{}/vendors/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_vendor_unauthenticated() {
    let client = test_client().await;

    let resp = client.post(format!("{}/vendors/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /vendors/options
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_options_both_appear() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let name_a = format!("OptA-{}", Uuid::new_v4());
    let name_b = format!("OptB-{}", Uuid::new_v4());
    let id_a = common::entities::create_vendor(&client, &name_a).await;
    let id_b = common::entities::create_vendor(&client, &name_b).await;

    let resp = client.get(format!("{}/vendors/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Assert plain array (not paginated)
    assert!(body.is_array(), "Options response should be a plain array");
    assert!(body.as_object().is_none(), "Options response should NOT be an object");

    let arr = body.as_array().unwrap();
    let opt_a = arr.iter().find(|v| v["id"].as_str().unwrap() == id_a).expect("option A");
    assert_eq!(opt_a["name"], name_a);
    let opt_b = arr.iter().find(|v| v["id"].as_str().unwrap() == id_b).expect("option B");
    assert_eq!(opt_b["name"], name_b);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_options_excludes_archived() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let active_name = format!("ActiveOpt-{}", Uuid::new_v4());
    let archived_name = format!("ArchivedOpt-{}", Uuid::new_v4());
    common::entities::create_vendor(&client, &active_name).await;
    let archived_id = common::entities::create_vendor(&client, &archived_name).await;

    // Archive the second vendor (noise data)
    client.post(format!("{}/vendors/{}/archive", V2_BASE, archived_id)).dispatch().await;

    let resp = client.get(format!("{}/vendors/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let arr = body.as_array().unwrap();
    let names: Vec<&str> = arr.iter().map(|v| v["name"].as_str().unwrap()).collect();
    assert!(names.contains(&active_name.as_str()), "Active vendor should appear in options");
    assert!(!names.contains(&archived_name.as_str()), "Archived vendor should NOT appear in options");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_options_unauthenticated() {
    let client = test_client().await;

    let resp = client.get(format!("{}/vendors/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// User isolation
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_user_isolation() {
    // User A creates a vendor
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;

    let vendor_name = format!("SecretVendor-{}", Uuid::new_v4());
    common::entities::create_vendor(&client_a, &vendor_name).await;

    // User B lists vendors — should NOT see User A's vendor
    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/vendors", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let data = body["data"].as_array().unwrap();
    let names: Vec<&str> = data.iter().map(|v| v["name"].as_str().unwrap()).collect();
    assert!(!names.contains(&vendor_name.as_str()), "User B should NOT see User A's vendor");
}
