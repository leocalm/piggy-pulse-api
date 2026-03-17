mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /accounts (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_checking_with_initial_balance() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "type": "Checking",
        "name": "Main Checking",
        "color": "#1a2b3c",
        "initialBalance": 50000,
        "currencyId": eur_id,
        "spendLimit": null
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_uuid(&body["id"]);
    assert_eq!(body["type"], "Checking");
    assert_eq!(body["name"], "Main Checking");
    assert_eq!(body["color"], "#1a2b3c");
    assert_eq!(body["initialBalance"], 50000);
    assert_eq!(body["status"], "active");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_credit_card_with_spend_limit() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "type": "CreditCard",
        "name": "Visa Gold",
        "color": "#ffd700",
        "initialBalance": 0,
        "currencyId": eur_id,
        "spendLimit": 200000
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["type"], "CreditCard");
    assert_eq!(body["name"], "Visa Gold");
    assert_eq!(body["spendLimit"], 200000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_allowance_with_null_spend_limit() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "type": "Allowance",
        "name": "Fun Money",
        "color": "#ff00ff",
        "initialBalance": 20000,
        "currencyId": eur_id,
        "spendLimit": null
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["type"], "Allowance");
    assert_eq!(body["initialBalance"], 20000);
    assert!(body["spendLimit"].is_null(), "expected spendLimit to be null for allowance");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_checking_with_spend_limit_rejected() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "type": "Checking",
        "name": "Bad Checking",
        "color": "#000000",
        "initialBalance": 10000,
        "currencyId": eur_id,
        "spendLimit": 100000
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_name_too_short() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "type": "Checking",
        "name": "AB",
        "color": "#000000",
        "initialBalance": 0,
        "currencyId": eur_id,
        "spendLimit": null
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_bad_color() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "type": "Checking",
        "name": "Bad Color Account",
        "color": "not-a-hex-color",
        "initialBalance": 0,
        "currencyId": eur_id,
        "spendLimit": null
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_missing_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "type": "Checking",
        "name": "Incomplete"
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
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
async fn test_create_account_no_auth() {
    let client = test_client().await;

    let payload = json!({
        "type": "Checking",
        "name": "Should Fail",
        "color": "#123456",
        "initialBalance": 0,
        "currencyId": Uuid::new_v4(),
        "spendLimit": null
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_account_returns_created_values() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    // Create with specific known values
    let payload = json!({
        "type": "Checking",
        "name": "My Detailed Account",
        "color": "#abc123",
        "initialBalance": 75000,
        "currencyId": eur_id,
        "spendLimit": null
    });
    let create_resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(create_resp.status(), Status::Created);
    let created: Value = serde_json::from_str(&create_resp.into_string().await.unwrap()).unwrap();
    let account_id = created["id"].as_str().unwrap();

    // GET and assert all values match creation
    let resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["id"], account_id);
    assert_eq!(body["name"], "My Detailed Account");
    assert_eq!(body["color"], "#abc123");
    assert_eq!(body["initialBalance"], 75000);
    assert_eq!(body["type"], "Checking");
    assert_eq!(body["status"], "active");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_account_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/accounts/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_account_user_isolation() {
    // User A creates an account
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let account_id = common::entities::create_account(&client_a, "Private", 10000).await;

    // User B tries to access it
    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_account_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_accounts_returns_created() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_account(&client, "Account Alpha", 10000).await;
    common::entities::create_account(&client, "Account Beta", 20000).await;

    let resp = client.get(format!("{}/accounts", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);

    let data = body["data"].as_array().unwrap();
    assert!(data.len() >= 2);
    assert_eq!(body["totalCount"].as_i64().unwrap(), data.len() as i64);

    let names: Vec<&str> = data.iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"Account Alpha"), "missing Account Alpha in {names:?}");
    assert!(names.contains(&"Account Beta"), "missing Account Beta in {names:?}");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_accounts_pagination_and_cursor() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create exactly 3 accounts for this fresh user
    for i in 1..=3 {
        common::entities::create_account(&client, &format!("Page Acct {}", i), 1000 * i).await;
    }

    // First page: limit=1
    let resp = client.get(format!("{}/accounts?limit=1", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["hasMore"], true);
    assert!(body["nextCursor"].is_string());

    // Cursor through all pages, collecting items
    let mut collected = body["data"].as_array().unwrap().clone();
    let mut next_cursor = body["nextCursor"].as_str().map(String::from);

    while let Some(cursor) = next_cursor {
        let resp = client.get(format!("{}/accounts?limit=1&cursor={}", V2_BASE, cursor)).dispatch().await;
        assert_eq!(resp.status(), Status::Ok);
        let page: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
        let page_data = page["data"].as_array().unwrap();
        assert!(!page_data.is_empty(), "non-final page returned empty data");
        collected.extend(page_data.iter().cloned());

        if page["hasMore"] == true {
            next_cursor = page["nextCursor"].as_str().map(String::from);
        } else {
            next_cursor = None;
        }
    }

    assert_eq!(collected.len(), 3, "expected 3 total items after paginating");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_accounts_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /accounts/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_account_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;
    let account_id = common::entities::create_account(&client, "Before Update", 10000).await;

    let payload = json!({
        "type": "Checking",
        "name": "After Update",
        "color": "#abcdef",
        "initialBalance": 20000,
        "currencyId": eur_id,
        "spendLimit": null
    });

    let resp = client
        .put(format!("{}/accounts/{}", V2_BASE, account_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Verify persistence via GET — not just the PUT response
    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "After Update");
    assert_eq!(body["color"], "#abcdef");
    assert_eq!(body["initialBalance"], 20000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_account_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "type": "Checking",
        "name": "Ghost Account",
        "color": "#000000",
        "initialBalance": 0,
        "currencyId": eur_id,
        "spendLimit": null
    });

    let resp = client
        .put(format!("{}/accounts/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_account_invalid_name() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;
    let account_id = common::entities::create_account(&client, "Valid Name", 10000).await;

    let payload = json!({
        "type": "Checking",
        "name": "AB",
        "color": "#000000",
        "initialBalance": 0,
        "currencyId": eur_id,
        "spendLimit": null
    });

    let resp = client
        .put(format!("{}/accounts/{}", V2_BASE, account_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_account_no_auth() {
    let client = test_client().await;

    let payload = json!({
        "type": "Checking",
        "name": "No Auth",
        "color": "#000000",
        "initialBalance": 0,
        "currencyId": Uuid::new_v4(),
        "spendLimit": null
    });

    let resp = client
        .put(format!("{}/accounts/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /accounts/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_account_then_get_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "To Delete", 5000).await;

    let resp = client.delete(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify deletion via GET
    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_account_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/accounts/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_account_no_auth() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/accounts/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /accounts/{id}/archive & /accounts/{id}/unarchive
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_sets_inactive() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "To Archive", 10000).await;

    let resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    // Verify status changed via GET
    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "inactive");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_already_archived_409() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Double Archive", 10000).await;

    // Archive once
    let first = client.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(first.status(), Status::Ok, "first archive should succeed");

    // Archive again — should conflict
    let second = client.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(second.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_then_unarchive_sets_active() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Archive Cycle", 10000).await;

    // Archive
    let archive_resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(archive_resp.status(), Status::Ok);

    // Unarchive
    let unarchive_resp = client.post(format!("{}/accounts/{}/unarchive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(unarchive_resp.status(), Status::Ok);

    // Verify status back to active via GET
    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "active");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_already_active_409() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Already Active", 10000).await;

    // Account is already active — unarchive should conflict
    let resp = client.post(format!("{}/accounts/{}/unarchive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/accounts/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/accounts/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /accounts/{id}/adjust-balance
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_adjust_balance_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Adjust Me", 10000).await;

    let payload = json!({ "newBalance": 50000 });

    let resp = client
        .post(format!("{}/accounts/{}/adjust-balance", V2_BASE, account_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Verify persistence via GET
    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["initialBalance"], 50000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_adjust_balance_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({ "newBalance": 50000 });

    let resp = client
        .post(format!("{}/accounts/{}/adjust-balance", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_adjust_balance_no_auth() {
    let client = test_client().await;

    let payload = json!({ "newBalance": 50000 });

    let resp = client
        .post(format!("{}/accounts/{}/adjust-balance", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts/{id}/details
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_details_reflects_transactions() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Arrange — specific known values
    let account_id = common::entities::create_account(&client, "Details Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Food", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 30_000, "2026-03-15").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 20_000, "2026-03-16").await;

    // Act
    let resp = client
        .get(format!("{}/accounts/{}/details?periodId={}", V2_BASE, account_id, period_id))
        .dispatch()
        .await;

    // Assert — values derived from arrangement
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["outflow"], 50_000); // 30_000 + 20_000
    assert_eq!(body["currentBalance"], 50_000); // 100_000 - 50_000
    assert_eq!(body["numberOfTransactions"], 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_details_no_transactions_zeros() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = common::entities::create_account(&client, "Empty Details", 80_000).await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/accounts/{}/details?periodId={}", V2_BASE, account_id, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["outflow"], 0);
    assert_eq!(body["inflow"], 0);
    assert_eq!(body["numberOfTransactions"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_details_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/accounts/{}/details?periodId={}", V2_BASE, Uuid::new_v4(), period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_details_without_period_id_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "No Period", 10_000).await;

    let resp = client.get(format!("{}/accounts/{}/details", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_details_no_auth() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/accounts/{}/details?periodId={}", V2_BASE, Uuid::new_v4(), Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts/{id}/balance-history
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_balance_history_reflects_transactions() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = common::entities::create_account(&client, "History Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Transport", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 30_000, "2026-03-10").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 20_000, "2026-03-20").await;

    let resp = client
        .get(format!("{}/accounts/{}/balance-history?periodId={}", V2_BASE, account_id, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let entries = body.as_array().unwrap();
    assert!(!entries.is_empty(), "balance history should contain entries");

    // Find entries on transaction dates and verify balances
    let entry_mar10 = entries
        .iter()
        .find(|e| e["date"].as_str() == Some("2026-03-10"))
        .expect("should have entry for 2026-03-10");
    assert_eq!(entry_mar10["balance"], 70_000); // 100_000 - 30_000

    let entry_mar20 = entries
        .iter()
        .find(|e| e["date"].as_str() == Some("2026-03-20"))
        .expect("should have entry for 2026-03-20");
    assert_eq!(entry_mar20["balance"], 50_000); // 70_000 - 20_000
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_balance_history_no_transactions_empty() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = common::entities::create_account(&client, "Empty History", 50_000).await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/accounts/{}/balance-history?periodId={}", V2_BASE, account_id, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let entries = body.as_array().unwrap();
    assert!(entries.is_empty(), "expected empty array when no transactions");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_balance_history_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/accounts/{}/balance-history?periodId={}", V2_BASE, Uuid::new_v4(), period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_balance_history_no_auth() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/accounts/{}/balance-history?periodId={}", V2_BASE, Uuid::new_v4(), Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts/options
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_returns_created_accounts() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let id_a = common::entities::create_account(&client, "Option Alpha", 10000).await;
    let id_b = common::entities::create_account(&client, "Option Beta", 20000).await;

    let resp = client.get(format!("{}/accounts/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Options is a plain array, not paginated
    let options = body.as_array().expect("expected plain array for options");
    assert!(options.len() >= 2);

    let opt_a = options
        .iter()
        .find(|o| o["id"].as_str() == Some(&id_a))
        .expect("Option Alpha not found in options");
    assert_eq!(opt_a["name"], "Option Alpha");
    assert!(opt_a["color"].is_string());

    let opt_b = options
        .iter()
        .find(|o| o["id"].as_str() == Some(&id_b))
        .expect("Option Beta not found in options");
    assert_eq!(opt_b["name"], "Option Beta");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_empty_state() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/accounts/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let options = body.as_array().expect("expected plain array");
    assert!(options.is_empty(), "fresh user should have no account options");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts/summary
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_summary_reflects_transactions() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Arrange
    let account_id = common::entities::create_account(&client, "Summary Acct", 100_000).await;
    let category_id = common::entities::create_category(&client, "Groceries", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    common::entities::create_transaction(&client, &account_id, &category_id, 40_000, "2026-03-10").await;

    // Act
    let resp = client.get(format!("{}/accounts/summary?periodId={}", V2_BASE, period_id)).dispatch().await;

    // Assert
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);

    let data = body["data"].as_array().unwrap();
    let acct = data
        .iter()
        .find(|a| a["id"].as_str() == Some(&account_id))
        .expect("account not found in summary");
    assert_eq!(acct["currentBalance"], 60_000); // 100_000 - 40_000
    assert_eq!(acct["netChangeThisPeriod"], -40_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_summary_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts/summary?periodId={}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
