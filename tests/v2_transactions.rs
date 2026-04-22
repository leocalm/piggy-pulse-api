mod common;

use common::auth::create_user_and_login;
use common::crypto::{decrypt_i64, decrypt_string};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// GET transaction list — returns a plain Vec<EncryptedTransactionResponse>.
async fn get_tx_list(client: &rocket::local::asynchronous::Client, url: &str) -> Vec<Value> {
    let resp = client.get(url.to_string()).dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "GET {} failed with {}", url, resp.status());
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    body.as_array().unwrap().clone()
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /transactions — create
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_regular_transaction_asserts_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Create Reg Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Groceries", "expense").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-10",
        "description": "Weekly groceries",
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

    common::assertions::assert_uuid(&body["id"]);
    assert_eq!(decrypt_i64(body["amountEnc"].as_str().unwrap()), 5000);
    assert_eq!(decrypt_string(body["descriptionEnc"].as_str().unwrap()), "Weekly groceries");
    assert_eq!(body["date"], "2026-03-10");
    assert!(body["firstCreatedAt"].as_str().is_some());
    assert_eq!(body["fromAccountId"], account_id);
    assert_eq!(body["categoryId"], category_id);
    assert!(body["vendorId"].is_null());
    assert!(body["toAccountId"].is_null());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transfer_transaction_asserts_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let from_id = common::entities::create_account(&client, "Transfer From", 100_000).await;
    let to_id = common::entities::create_account(&client, "Transfer To", 0).await;
    let category_id = common::entities::create_category(&client, "Internal Transfer", "transfer").await;

    let payload = serde_json::json!({
        "transactionType": "Transfer",
        "date": "2026-03-15",
        "description": "Move funds",
        "amount": 25_000,
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

    common::assertions::assert_uuid(&body["id"]);
    assert_eq!(decrypt_i64(body["amountEnc"].as_str().unwrap()), 25_000);
    assert_eq!(decrypt_string(body["descriptionEnc"].as_str().unwrap()), "Move funds");
    assert_eq!(body["date"], "2026-03-15");
    assert_eq!(body["fromAccountId"], from_id);
    assert_eq!(body["categoryId"], category_id);
    assert_eq!(body["toAccountId"], to_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_with_vendor() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Vendor Tx Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Shopping", "expense").await;
    let vendor_id = common::entities::create_vendor(&client, "Amazon Store").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-12",
        "description": "Online purchase",
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
    assert_eq!(decrypt_i64(body["amountEnc"].as_str().unwrap()), 3500);
    assert_eq!(body["vendorId"], vendor_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_regular_with_to_account_id_returns_regular() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Reg Bad Acct", 100_000).await;
    let _to_id = common::entities::create_account(&client, "Reg Bad To", 0).await;
    let category_id = common::entities::create_category(&client, "RegBadCat", "expense").await;

    // Regular + toAccountId: serde ignores unknown fields for the Regular variant
    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "Bad regular",
        "amount": 1000,
        "fromAccountId": account_id,
        "categoryId": category_id,
        "vendorId": null,
        "toAccountId": "00000000-0000-0000-0000-000000000000"
    });

    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    if resp.status() == Status::Created {
        let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
        assert!(body["toAccountId"].is_null(), "Regular tx must not have toAccountId");
    } else {
        assert_eq!(resp.status(), Status::BadRequest);
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transfer_without_to_account_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Xfer No To", 100_000).await;
    let category_id = common::entities::create_category(&client, "Xfer No To Cat", "transfer").await;

    let payload = serde_json::json!({
        "transactionType": "Transfer",
        "date": "2026-03-01",
        "description": "Missing toAccountId",
        "amount": 1000,
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

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400/422, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_nonexistent_from_account_returns_400() {
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
async fn test_create_transaction_nonexistent_category_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Ghost Cat Acct", 100_000).await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "Ghost category",
        "amount": 5000,
        "fromAccountId": account_id,
        "categoryId": Uuid::new_v4(),
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
async fn test_create_transaction_missing_fields_returns_400() {
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
async fn test_create_transaction_unauthenticated_returns_401() {
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
// GET /transactions — list (plain array of EncryptedTransactionResponse)
// Filter query params (accountId, categoryId, vendorId, direction, limit,
// cursor, fromDate, toDate) are not yet wired in the encrypted service layer.
// Only periodId filtering is active.
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_returns_created() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "List Tx Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "List Tx Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction(&client, &account_id, &category_id, 1000, "2026-03-05").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 2000, "2026-03-10").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 3000, "2026-03-15").await;

    let data = get_tx_list(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;

    assert_eq!(data.len(), 3);
    let mut amounts: Vec<i64> = data.iter().map(|t| decrypt_i64(t["amountEnc"].as_str().unwrap())).collect();
    amounts.sort();
    assert_eq!(amounts, vec![1000, 2000, 3000]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/transactions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client.get(format!("{}/transactions?periodId={}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_empty_period_returns_empty() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-06-01", "2026-06-30").await;

    let data = get_tx_list(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;

    assert!(data.is_empty(), "empty period should return empty array");
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /transactions/{id} — update
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_amount_persists_via_list() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Upd Amt Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Upd Amt Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 5000, "2026-03-10").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-10",
        "description": "Updated purchase",
        "amount": 9999,
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
    let put_body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(decrypt_i64(put_body["amountEnc"].as_str().unwrap()), 9999);

    // Verify persistence via GET list
    let data = get_tx_list(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;
    let found = data.iter().find(|t| t["id"].as_str().unwrap() == tx_id);
    assert!(found.is_some(), "updated transaction should appear in list");
    assert_eq!(decrypt_i64(found.unwrap()["amountEnc"].as_str().unwrap()), 9999);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_description_persists_via_list() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Upd Desc Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Upd Desc Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 5000, "2026-03-10").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-10",
        "description": "New description text",
        "amount": 5000,
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

    let data = get_tx_list(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;
    let found = data.iter().find(|t| t["id"].as_str().unwrap() == tx_id);
    assert!(found.is_some());
    assert_eq!(decrypt_string(found.unwrap()["descriptionEnc"].as_str().unwrap()), "New description text");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_nonexistent_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Upd 404 Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Upd 404 Cat", "expense").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "Ghost update",
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
async fn test_update_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/transactions/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "transactionType": "Regular",
                "date": "2026-03-01",
                "description": "No auth update",
                "amount": 1000,
                "fromAccountId": Uuid::new_v4(),
                "categoryId": Uuid::new_v4(),
                "vendorId": null
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /transactions/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_then_verify_gone_via_list() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Del Verify Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Del Verify Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    let tx_id = common::entities::create_transaction(&client, &account_id, &category_id, 5000, "2026-03-10").await;

    let data_before = get_tx_list(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;
    let before_ids: Vec<&str> = data_before.iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert!(before_ids.contains(&tx_id.as_str()), "tx should exist before delete");

    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, tx_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    let data_after = get_tx_list(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;
    let after_ids: Vec<&str> = data_after.iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert!(!after_ids.contains(&tx_id.as_str()), "tx should be gone after delete");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_nonexistent_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Cross-domain isolation
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_user_isolation_user_b_cannot_see_user_a_transactions() {
    // User A
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let acct_a = common::entities::create_account(&client_a, "IsoAcctA", 100_000).await;
    let cat_a = common::entities::create_category(&client_a, "IsoCatA", "expense").await;
    let _period_a = common::entities::create_period(&client_a, "2026-03-01", "2026-03-31").await;
    common::entities::create_transaction(&client_a, &acct_a, &cat_a, 9999, "2026-03-10").await;

    // User B
    let client_b = test_client().await;
    create_user_and_login(&client_b).await;
    let period_b = common::entities::create_period(&client_b, "2026-03-01", "2026-03-31").await;

    let data = get_tx_list(&client_b, &format!("{}/transactions?periodId={}", V2_BASE, period_b)).await;

    assert!(data.is_empty(), "User B should not see User A's transactions");
}
