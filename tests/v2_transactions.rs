mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// GET helper that returns JSON body.
async fn get_json(client: &rocket::local::asynchronous::Client, url: &str) -> Value {
    let resp = client.get(url.to_string()).dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "GET {} failed with {}", url, resp.status());
    serde_json::from_str(&resp.into_string().await.unwrap()).unwrap()
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

    // Assert ALL scalar fields
    common::assertions::assert_uuid(&body["id"]);
    assert_eq!(body["amount"], 5000);
    assert_eq!(body["description"], "Weekly groceries");
    assert_eq!(body["date"], "2026-03-10");
    assert_eq!(body["transactionType"], "regular");

    // fromAccount ref
    assert_eq!(body["fromAccount"]["id"], account_id);
    assert_eq!(body["fromAccount"]["name"], "Create Reg Acct");
    assert!(body["fromAccount"]["color"].is_string());

    // category ref
    assert_eq!(body["category"]["id"], category_id);
    assert_eq!(body["category"]["name"], "Groceries");
    assert_eq!(body["category"]["type"], "expense");
    assert!(body["category"]["color"].is_string());
    assert!(body["category"]["icon"].is_string());

    // vendor is null for this transaction
    assert!(body["vendor"].is_null());

    // toAccount is null for regular
    assert!(body["toAccount"].is_null());
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
    assert_eq!(body["amount"], 25_000);
    assert_eq!(body["description"], "Move funds");
    assert_eq!(body["date"], "2026-03-15");
    assert_eq!(body["transactionType"], "transfer");
    assert_eq!(body["fromAccount"]["id"], from_id);
    assert_eq!(body["category"]["id"], category_id);
    assert_eq!(body["category"]["type"], "transfer");

    // toAccount must be populated for transfer
    assert_eq!(body["toAccount"]["id"], to_id);
    assert_eq!(body["toAccount"]["name"], "Transfer To");
    assert!(body["toAccount"]["color"].is_string());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_with_vendor_asserts_vendor_fields() {
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

    assert_eq!(body["amount"], 3500);
    assert_eq!(body["vendor"]["id"], vendor_id);
    assert_eq!(body["vendor"]["name"], "Amazon Store");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_regular_with_to_account_id_returns_regular() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Reg Bad Acct", 100_000).await;
    let to_id = common::entities::create_account(&client, "Reg Bad To", 0).await;
    let category_id = common::entities::create_category(&client, "RegBadCat", "expense").await;

    // Regular + toAccountId: serde ignores unknown fields for the Regular variant
    // so this creates a regular transaction with no toAccount
    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-03-01",
        "description": "Bad regular",
        "amount": 1000,
        "fromAccountId": account_id,
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

    // serde internally-tagged enum ignores unknown fields for Regular variant
    if resp.status() == Status::Created {
        let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
        assert_eq!(body["transactionType"], "regular");
        assert!(body["toAccount"].is_null(), "Regular tx must not have toAccount");
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

    // Transfer variant requires toAccountId -- serde deserialization will fail
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
// GET /transactions -- list
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_three_items_correct_amounts() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "List 3 Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "List 3 Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction(&client, &account_id, &category_id, 1000, "2026-03-05").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 2000, "2026-03-10").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 3000, "2026-03-15").await;

    let body = get_json(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;

    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 3);

    let mut amounts: Vec<i64> = data.iter().map(|t| t["amount"].as_i64().unwrap()).collect();
    amounts.sort();
    assert_eq!(amounts, vec![1000, 2000, 3000]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_filter_by_direction_expense() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Dir Filter Acct", 100_000).await;
    let expense_cat = common::entities::create_category(&client, "Food", "expense").await;
    let income_cat = common::entities::create_category(&client, "Salary", "income").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction(&client, &account_id, &expense_cat, 5000, "2026-03-05").await;
    // noise: income transaction
    common::entities::create_transaction(&client, &account_id, &income_cat, 10_000, "2026-03-06").await;

    let body = get_json(&client, &format!("{}/transactions?periodId={}&direction=expense", V2_BASE, period_id)).await;

    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1, "only expense tx should be returned");
    assert_eq!(data[0]["amount"], 5000);
    assert_eq!(data[0]["category"]["type"], "expense");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_filter_by_account_id() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let acct_a = common::entities::create_account(&client, "AcctFilter A", 100_000).await;
    let acct_b = common::entities::create_account(&client, "AcctFilter B", 100_000).await;
    let category_id = common::entities::create_category(&client, "AcctFilter Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction(&client, &acct_a, &category_id, 7777, "2026-03-10").await;
    // noise: different account
    common::entities::create_transaction(&client, &acct_b, &category_id, 8888, "2026-03-10").await;

    let body = get_json(&client, &format!("{}/transactions?periodId={}&accountId={}", V2_BASE, period_id, acct_a)).await;

    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["amount"], 7777);
    assert_eq!(data[0]["fromAccount"]["id"], acct_a);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_filter_by_category_id() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "CatFilter Acct", 100_000).await;
    let cat_a = common::entities::create_category(&client, "CatFilterA", "expense").await;
    let cat_b = common::entities::create_category(&client, "CatFilterB", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction(&client, &account_id, &cat_a, 4444, "2026-03-10").await;
    // noise: different category
    common::entities::create_transaction(&client, &account_id, &cat_b, 5555, "2026-03-10").await;

    let body = get_json(&client, &format!("{}/transactions?periodId={}&categoryId={}", V2_BASE, period_id, cat_a)).await;

    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["amount"], 4444);
    assert_eq!(data[0]["category"]["id"], cat_a);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_filter_by_vendor_id() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "VendorFilter Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "VendorFilter Cat", "expense").await;
    let vendor_a = common::entities::create_vendor(&client, "Vendor Alpha").await;
    let vendor_b = common::entities::create_vendor(&client, "Vendor Beta").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction_with_vendor(&client, &account_id, &category_id, 6666, "2026-03-10", &vendor_a).await;
    // noise: different vendor
    common::entities::create_transaction_with_vendor(&client, &account_id, &category_id, 7777, "2026-03-10", &vendor_b).await;

    let body = get_json(&client, &format!("{}/transactions?periodId={}&vendorId={}", V2_BASE, period_id, vendor_a)).await;

    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["amount"], 6666);
    assert_eq!(data[0]["vendor"]["id"], vendor_a);
    assert_eq!(data[0]["vendor"]["name"], "Vendor Alpha");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_filter_by_date_range() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "DateFilter Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "DateFilter Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction(&client, &account_id, &category_id, 1111, "2026-03-05").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 2222, "2026-03-15").await;
    // noise: outside date range
    common::entities::create_transaction(&client, &account_id, &category_id, 3333, "2026-03-25").await;

    let body = get_json(
        &client,
        &format!("{}/transactions?periodId={}&fromDate=2026-03-01&toDate=2026-03-20", V2_BASE, period_id),
    )
    .await;

    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    let mut amounts: Vec<i64> = data.iter().map(|t| t["amount"].as_i64().unwrap()).collect();
    amounts.sort();
    assert_eq!(amounts, vec![1111, 2222]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_pagination_limit_1() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Page Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Page Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction(&client, &account_id, &category_id, 1000, "2026-03-05").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 2000, "2026-03-10").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 3000, "2026-03-15").await;

    let body = get_json(&client, &format!("{}/transactions?periodId={}&limit=1", V2_BASE, period_id)).await;

    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(body["hasMore"], true);
    assert!(body["nextCursor"].is_string(), "nextCursor should be present when hasMore");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_transactions_cursor_through_all_pages() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "CursorAll Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "CursorAll Cat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    common::entities::create_transaction(&client, &account_id, &category_id, 1000, "2026-03-05").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 2000, "2026-03-10").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 3000, "2026-03-15").await;

    let mut collected_amounts: Vec<i64> = Vec::new();
    let mut cursor: Option<String> = None;
    let mut pages = 0;

    loop {
        let url = match &cursor {
            Some(c) => format!("{}/transactions?periodId={}&limit=1&cursor={}", V2_BASE, period_id, c),
            None => format!("{}/transactions?periodId={}&limit=1", V2_BASE, period_id),
        };

        let body = get_json(&client, &url).await;
        let data = body["data"].as_array().unwrap();

        for item in data {
            collected_amounts.push(item["amount"].as_i64().unwrap());
        }

        pages += 1;
        if pages > 10 {
            panic!("Pagination infinite loop");
        }

        if body["hasMore"].as_bool().unwrap_or(false) {
            cursor = Some(body["nextCursor"].as_str().unwrap().to_string());
        } else {
            break;
        }
    }

    collected_amounts.sort();
    assert_eq!(collected_amounts, vec![1000, 2000, 3000]);
    assert_eq!(pages, 3, "should take exactly 3 pages with limit=1 and 3 items");
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

    let body = get_json(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;

    let data = body["data"].as_array().unwrap();
    assert!(data.is_empty(), "empty period should return empty data array");
    assert_eq!(body["hasMore"], false);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /transactions/{id} -- update
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_amount_persists_via_get() {
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
    assert_eq!(put_body["amount"], 9999);

    // Verify persistence via GET list
    let list_body = get_json(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;

    let data = list_body["data"].as_array().unwrap();
    let found = data.iter().find(|t| t["id"].as_str().unwrap() == tx_id);
    assert!(found.is_some(), "updated transaction should appear in list");
    assert_eq!(found.unwrap()["amount"], 9999);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_description_persists_via_get() {
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

    // Verify persistence via GET list
    let list_body = get_json(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;

    let data = list_body["data"].as_array().unwrap();
    let found = data.iter().find(|t| t["id"].as_str().unwrap() == tx_id);
    assert!(found.is_some());
    assert_eq!(found.unwrap()["description"], "New description text");
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

    // Confirm it exists
    let list_before = get_json(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;
    let before_ids: Vec<&str> = list_before["data"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert!(before_ids.contains(&tx_id.as_str()), "tx should exist before delete");

    // Delete
    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, tx_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify gone
    let list_after = get_json(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;
    let after_ids: Vec<&str> = list_after["data"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap()).collect();
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

    let body = get_json(&client_b, &format!("{}/transactions?periodId={}", V2_BASE, period_b)).await;

    let data = body["data"].as_array().unwrap();
    assert!(data.is_empty(), "User B should not see User A's transactions");
}
