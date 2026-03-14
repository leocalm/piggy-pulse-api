mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /accounts (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_checking_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
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
    assert_eq!(body["type"], "Checking");
    assert_eq!(body["name"], "Main Checking");
    assert_eq!(body["color"], "#1a2b3c");
    assert_eq!(body["initialBalance"], 50000);
    assert_eq!(body["status"], "active");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_savings_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
        "type": "Savings",
        "name": "Emergency Fund",
        "color": "#00ff00",
        "initialBalance": 100000,
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
    assert_eq!(body["type"], "Savings");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_credit_card_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
        "type": "CreditCard",
        "name": "Visa Gold",
        "color": "#ffd700",
        "initialBalance": 0,
        "currencyId": eur_id,
        "spendLimit": 500000
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
    assert_eq!(body["spendLimit"], 500000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_wallet_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
        "type": "Wallet",
        "name": "Cash Wallet",
        "color": "#abcdef",
        "initialBalance": 5000,
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
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_allowance_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
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
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_name_too_short() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
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

    let payload = serde_json::json!({
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

    let payload = serde_json::json!({
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
async fn test_create_account_malformed_json() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body("{invalid json")
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

    let payload = serde_json::json!({
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
async fn test_get_account_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Get Me", 10000).await;

    let resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["id"], account_id);
    assert_eq!(body["name"], "Get Me");
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
async fn test_get_account_wrong_user() {
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
async fn test_list_accounts_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_account(&client, "Account One", 10000).await;
    common::entities::create_account(&client, "Account Two", 20000).await;

    let resp = client.get(format!("{}/accounts", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
    assert!(body["data"].as_array().unwrap().len() >= 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_accounts_pagination() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create 3 accounts, request with limit=1
    for i in 0..3 {
        common::entities::create_account(&client, &format!("Paginated {}", i), 1000).await;
    }

    let resp = client.get(format!("{}/accounts?limit=1", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["hasMore"], true);
    assert!(body["nextCursor"].is_string());
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
async fn test_update_account_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;
    let account_id = common::entities::create_account(&client, "Before Update", 10000).await;

    let payload = serde_json::json!({
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
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "After Update");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_account_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
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
async fn test_update_account_validation() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;
    let account_id = common::entities::create_account(&client, "Valid Account", 10000).await;

    let payload = serde_json::json!({
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

    let payload = serde_json::json!({
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
async fn test_delete_account_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "To Delete", 0).await;

    let resp = client.delete(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NoContent);

    // Verify it's gone
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
async fn test_archive_account_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "To Archive", 10000).await;

    let resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_account_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_account_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_account_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "To Unarchive", 10000).await;

    // Archive first
    client.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;

    let resp = client.post(format!("{}/accounts/{}/unarchive", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_account_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/accounts/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_account_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/accounts/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /accounts/{id}/adjust-balance
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_adjust_balance_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Balance Adj", 10000).await;

    let payload = serde_json::json!({
        "newBalance": 50000
    });

    let resp = client
        .post(format!("{}/accounts/{}/adjust-balance", V2_BASE, account_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_adjust_balance_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "newBalance": 50000
    });

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

    let payload = serde_json::json!({
        "newBalance": 50000
    });

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
async fn test_account_details_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Details Acct", 10000).await;

    let resp = client.get(format!("{}/accounts/{}/details", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_account_details_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/accounts/{}/details", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_account_details_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts/{}/details", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts/{id}/balance-history
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_balance_history_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "History Acct", 10000).await;

    let resp = client.get(format!("{}/accounts/{}/balance-history", V2_BASE, account_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_balance_history_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/accounts/{}/balance-history", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_balance_history_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts/{}/balance-history", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts/options
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_account_options_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_account(&client, "Option Acct", 10000).await;

    let resp = client.get(format!("{}/accounts/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body.is_array());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_account_options_empty() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/accounts/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body.is_array());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_account_options_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /accounts/summary
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_account_summary_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_account(&client, "Summary Acct", 10000).await;

    let resp = client.get(format!("{}/accounts/summary", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_account_summary_empty() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/accounts/summary", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_account_summary_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts/summary", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
