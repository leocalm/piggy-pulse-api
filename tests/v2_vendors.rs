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
async fn test_create_vendor_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "name": "Supermarket",
        "description": "Weekly groceries"
    });

    let resp = client
        .post(format!("{}/vendors", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "Supermarket");
    assert_eq!(body["status"], "active");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_vendor_name_too_short() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "name": "AB",
        "description": null
    });

    let resp = client
        .post(format!("{}/vendors", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_vendor_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "name": "No Auth Vendor",
        "description": null
    });

    let resp = client
        .post(format!("{}/vendors", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /vendors (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_vendors_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_vendor(&client, "Vendor One").await;

    let resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_vendors_pagination() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    for i in 0..3 {
        common::entities::create_vendor(&client, &format!("PageVendor {}", i)).await;
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
async fn test_list_vendors_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/vendors", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /vendors/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_vendor_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let vendor_id = common::entities::create_vendor(&client, "Old Vendor").await;

    let payload = serde_json::json!({
        "name": "New Vendor",
        "description": "Updated description"
    });

    let resp = client
        .put(format!("{}/vendors/{}", V2_BASE, vendor_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "New Vendor");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_vendor_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "name": "Ghost Vendor",
        "description": null
    });

    let resp = client
        .put(format!("{}/vendors/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_vendor_no_auth() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/vendors/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body("{}")
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /vendors/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_vendor_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let vendor_id = common::entities::create_vendor(&client, "To Delete").await;

    let resp = client.delete(format!("{}/vendors/{}", V2_BASE, vendor_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NoContent);
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
async fn test_delete_vendor_no_auth() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/vendors/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /vendors/{id}/archive & unarchive
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_vendor_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let vendor_id = common::entities::create_vendor(&client, "Archive Me").await;

    let resp = client.post(format!("{}/vendors/{}/archive", V2_BASE, vendor_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
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
async fn test_archive_vendor_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/vendors/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_vendor_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let vendor_id = common::entities::create_vendor(&client, "Unarchive Me").await;

    client.post(format!("{}/vendors/{}/archive", V2_BASE, vendor_id)).dispatch().await;

    let resp = client.post(format!("{}/vendors/{}/unarchive", V2_BASE, vendor_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_vendor_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/vendors/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /vendors/options
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_options_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_vendor(&client, "Option Vendor").await;

    let resp = client.get(format!("{}/vendors/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body.is_array());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_vendor_options_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/vendors/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
