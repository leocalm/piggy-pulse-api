mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /transactions (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_regular_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Tx Account", 100000).await;
    let category_id = common::entities::create_category(&client, "Tx Category", "expense").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "Test purchase",
        "amount": 5000,
        "fromAccountId": account_id,
        "categoryId": category_id,
        "vendorId": null
    });

    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["amount"], 5000);
    assert_eq!(body["transactionType"], "Regular");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_transfer_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let from_id = common::entities::create_account(&client, "From Account", 100000).await;
    let to_id = common::entities::create_account(&client, "To Account", 0).await;
    let category_id = common::entities::create_category(&client, "Transfer Cat", "transfer").await;

    let payload = serde_json::json!({
        "transactionType": "Transfer",
        "date": "2026-03-01",
        "description": "Internal transfer",
        "amount": 25000,
        "fromAccountId": from_id,
        "categoryId": category_id,
        "vendorId": null,
        "toAccountId": to_id
    });

    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["transactionType"], "Transfer");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_with_vendor() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Vendor Tx Acct", 100000).await;
    let category_id = common::entities::create_category(&client, "Vendor Tx Cat", "expense").await;
    let vendor_id = common::entities::create_vendor(&client, "Test Store").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "Store purchase",
        "amount": 3500,
        "fromAccountId": account_id,
        "categoryId": category_id,
        "vendorId": vendor_id
    });

    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["vendor"].is_object());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_missing_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "amount": 5000
    });

    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400 or 422, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_nonexistent_account() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let category_id = common::entities::create_category(&client, "Ghost Acct Cat", "expense").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "Ghost account",
        "amount": 5000,
        "fromAccountId": Uuid::new_v4(),
        "categoryId": category_id,
        "vendorId": null
    });

    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::NotFound,
        "expected 400 or 404, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "No auth",
        "amount": 5000,
        "fromAccountId": Uuid::new_v4(),
        "categoryId": Uuid::new_v4(),
        "vendorId": null
    });

    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /transactions (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "List Tx Acct", 100000).await;
    let category_id = common::entities::create_category(&client, "List Tx Cat", "expense").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 5000, "2026-03-01").await;

    let resp = client.get(format!("{}/transactions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_with_filters() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Filter Acct", 100000).await;
    let category_id = common::entities::create_category(&client, "Filter Cat", "expense").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 5000, "2026-03-01").await;

    let resp = client
        .get(format!("{}/transactions?direction=expense&fromDate=2026-03-01&toDate=2026-03-31", V2_BASE))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_pagination() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Page Tx Acct", 100000).await;
    let category_id = common::entities::create_category(&client, "Page Tx Cat", "expense").await;
    for i in 1..=3 {
        common::entities::create_transaction(&client, &account_id, &category_id, 1000 * i, &format!("2026-03-0{}", i)).await;
    }

    let resp = client.get(format!("{}/transactions?limit=1", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["hasMore"], true);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/transactions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /transactions/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_transaction_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Upd Tx Acct", 100000).await;
    let category_id = common::entities::create_category(&client, "Upd Tx Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 5000, "2026-03-01").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-02",
        "description": "Updated purchase",
        "amount": 7500,
        "fromAccountId": account_id,
        "categoryId": category_id,
        "vendorId": null
    });

    let resp = client
        .put(format!("{}/transactions/{}", V2_BASE, tx_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["amount"], 7500);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_transaction_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "NotFound Tx Acct", 100000).await;
    let category_id = common::entities::create_category(&client, "NotFound Tx Cat", "expense").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "Ghost",
        "amount": 1000,
        "fromAccountId": account_id,
        "categoryId": category_id,
        "vendorId": null
    });

    let resp = client
        .put(format!("{}/transactions/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_transaction_no_auth() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/transactions/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body("{}")
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /transactions/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_transaction_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Del Tx Acct", 100000).await;
    let category_id = common::entities::create_category(&client, "Del Tx Cat", "expense").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 5000, "2026-03-01").await;

    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, tx_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NoContent);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_transaction_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_transaction_no_auth() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
