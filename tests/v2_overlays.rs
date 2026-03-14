mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /overlays (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "name": "Trip Budget",
        "icon": null,
        "startDate": "2026-03-01",
        "endDate": "2026-03-15",
        "inclusionMode": "manual",
        "totalCapAmount": 200000,
        "categoryCaps": [],
        "rules": null
    });

    let resp = client
        .post(format!("{}/overlays", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "Trip Budget");
    assert_eq!(body["totalCapAmount"], 200000);
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_with_rules() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let category_id = common::entities::create_category(&client, "Overlay Cat", "expense").await;

    let payload = serde_json::json!({
        "name": "Rules Overlay",
        "icon": null,
        "startDate": "2026-04-01",
        "endDate": "2026-04-30",
        "inclusionMode": "rules",
        "totalCapAmount": null,
        "categoryCaps": [{
            "categoryId": category_id,
            "capAmount": 50000
        }],
        "rules": {
            "categoryIds": [category_id]
        }
    });

    let resp = client
        .post(format!("{}/overlays", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["inclusionMode"], "rules");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_overlay_missing_name() {
    let client = test_client().await;
    create_user_and_login(&client).await;

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
async fn test_list_overlays_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_overlay(&client, "List Overlay", "2026-03-01", "2026-03-31").await;

    let resp = client.get(format!("{}/overlays", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
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
async fn test_get_overlay_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let overlay_id = common::entities::create_overlay(&client, "Get Me", "2026-05-01", "2026-05-31").await;

    let resp = client.get(format!("{}/overlays/{}", V2_BASE, overlay_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["id"], overlay_id);
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
async fn test_update_overlay_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let overlay_id = common::entities::create_overlay(&client, "Before Update", "2026-06-01", "2026-06-30").await;

    let payload = serde_json::json!({
        "name": "After Update",
        "icon": null,
        "startDate": "2026-06-01",
        "endDate": "2026-06-30",
        "inclusionMode": "manual",
        "totalCapAmount": 300000,
        "categoryCaps": [],
        "rules": null
    });

    let resp = client
        .put(format!("{}/overlays/{}", V2_BASE, overlay_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "After Update");
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
        .body("{}")
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /overlays/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_overlay_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let overlay_id = common::entities::create_overlay(&client, "To Delete", "2026-07-01", "2026-07-31").await;

    let resp = client.delete(format!("{}/overlays/{}", V2_BASE, overlay_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NoContent);
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
async fn test_overlay_transactions_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let overlay_id = common::entities::create_overlay(&client, "Tx Overlay", "2026-03-01", "2026-03-31").await;

    let resp = client.get(format!("{}/overlays/{}/transactions", V2_BASE, overlay_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
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
// DELETE /overlays/{id}/transactions/{txId}/exclude
// ═══════════════════════════════════════════════════════════════════════════════

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
