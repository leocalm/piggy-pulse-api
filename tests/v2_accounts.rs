mod common;

use common::auth::create_user_and_login;
use common::crypto::{decrypt_i64, decrypt_string};
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
        "accountType": "checking",
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
    assert_eq!(body["accountType"], "checking");
    assert_eq!(decrypt_string(body["nameEnc"].as_str().unwrap()), "Main Checking");
    assert_eq!(decrypt_string(body["colorEnc"].as_str().unwrap()), "#1a2b3c");
    assert_eq!(decrypt_i64(body["currentBalanceEnc"].as_str().unwrap()), 50000);
    assert_eq!(body["status"], "active");
    assert_eq!(body["currencyId"], eur_id);
    assert!(body["spendLimitEnc"].is_null(), "checking accounts should have null spendLimitEnc");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_credit_card_with_spend_limit() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "accountType": "creditcard",
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
    assert_eq!(body["accountType"], "creditcard");
    assert_eq!(decrypt_string(body["nameEnc"].as_str().unwrap()), "Visa Gold");
    assert_eq!(decrypt_string(body["colorEnc"].as_str().unwrap()), "#ffd700");
    assert_eq!(decrypt_i64(body["currentBalanceEnc"].as_str().unwrap()), 0);
    assert_eq!(decrypt_i64(body["spendLimitEnc"].as_str().unwrap()), 200000);
    assert_eq!(body["status"], "active");
    assert_eq!(body["currencyId"], eur_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_allowance_with_null_spend_limit() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "accountType": "allowance",
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
    assert_eq!(body["accountType"], "allowance");
    assert_eq!(decrypt_string(body["nameEnc"].as_str().unwrap()), "Fun Money");
    assert_eq!(decrypt_string(body["colorEnc"].as_str().unwrap()), "#ff00ff");
    assert_eq!(decrypt_i64(body["currentBalanceEnc"].as_str().unwrap()), 20000);
    assert!(body["spendLimitEnc"].is_null(), "expected spendLimitEnc to be null for allowance");
    assert_eq!(body["status"], "active");
    assert_eq!(body["currencyId"], eur_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_no_auth() {
    let client = test_client().await;

    let payload = json!({
        "accountType": "checking",
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
        "accountType": "checking",
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
    assert_eq!(decrypt_string(body["nameEnc"].as_str().unwrap()), "My Detailed Account");
    assert_eq!(decrypt_string(body["colorEnc"].as_str().unwrap()), "#abc123");
    assert_eq!(decrypt_i64(body["currentBalanceEnc"].as_str().unwrap()), 75000);
    assert_eq!(body["accountType"], "checking");
    assert_eq!(body["status"], "active");
    assert_eq!(body["currencyId"], eur_id);
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
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let account_id = common::entities::create_account(&client_a, "Private", 10000).await;

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
    assert_eq!(data.len(), 2);
    assert_eq!(body["totalCount"].as_i64().unwrap(), 2);

    let names: Vec<String> = data.iter().map(|a| decrypt_string(a["nameEnc"].as_str().unwrap())).collect();
    assert!(names.contains(&"Account Alpha".to_string()), "missing Account Alpha in {names:?}");
    assert!(names.contains(&"Account Beta".to_string()), "missing Account Beta in {names:?}");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_accounts_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/accounts", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// User isolation — write operations
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_account_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let account_id = common::entities::create_account(&client_a, "User A Only", 10000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;
    let eur_id = common::auth::get_eur_currency_id(&client_b).await;

    let payload = json!({
        "accountType": "checking",
        "name": "Hijacked",
        "color": "#000000",
        "initialBalance": 99999,
        "currencyId": eur_id,
        "spendLimit": null
    });
    let resp = client_b
        .put(format!("{}/accounts/{}", V2_BASE, account_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);

    // Verify User A's account is unchanged
    let get_resp = client_a.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(decrypt_string(body["nameEnc"].as_str().unwrap()), "User A Only");
    assert_eq!(decrypt_i64(body["currentBalanceEnc"].as_str().unwrap()), 10000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_account_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let account_id = common::entities::create_account(&client_a, "Protected", 10000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.delete(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);

    // Verify User A's account still exists
    let get_resp = client_a.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_account_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let account_id = common::entities::create_account(&client_a, "No Archive", 10000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);

    // Verify still active
    let get_resp = client_a.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "active");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_adjust_balance_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let account_id = common::entities::create_account(&client_a, "Stable Balance", 10000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let payload = json!({ "newBalance": 99999 });
    let resp = client_b
        .post(format!("{}/accounts/{}/adjust-balance", V2_BASE, account_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);

    // Verify balance and status unchanged
    let get_resp = client_a.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(decrypt_i64(body["currentBalanceEnc"].as_str().unwrap()), 10000);
    assert_eq!(body["status"], "active");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_account_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let account_id = common::entities::create_account(&client_a, "No Unarchive", 10000).await;

    // Archive it so unarchive is a valid operation
    let archive_resp = client_a.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(archive_resp.status(), Status::NoContent);

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.post(format!("{}/accounts/{}/unarchive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);

    // Verify still archived
    let get_resp = client_a.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "inactive");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_accounts_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    common::entities::create_account(&client_a, "User A Account", 10000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/accounts", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["totalCount"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    common::entities::create_account(&client_a, "User A Option", 10000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/accounts/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body.as_array().unwrap().is_empty(), "User B should see no options from User A");
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
        "accountType": "checking",
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
    let put_body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(decrypt_string(put_body["nameEnc"].as_str().unwrap()), "After Update");
    assert_eq!(decrypt_string(put_body["colorEnc"].as_str().unwrap()), "#abcdef");
    assert_eq!(put_body["accountType"], "checking");
    assert_eq!(put_body["currencyId"], eur_id);

    // Also verify persistence via GET
    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(decrypt_string(body["nameEnc"].as_str().unwrap()), "After Update");
    assert_eq!(decrypt_string(body["colorEnc"].as_str().unwrap()), "#abcdef");
    assert_eq!(body["accountType"], "checking");
    assert_eq!(body["currencyId"], eur_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_account_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "accountType": "checking",
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
async fn test_update_account_no_auth() {
    let client = test_client().await;

    let payload = json!({
        "accountType": "checking",
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
    assert_eq!(resp.status(), Status::NoContent);

    // Verify status changed via GET
    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "inactive");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_then_unarchive_sets_active() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Archive Cycle", 10000).await;

    // Archive
    let archive_resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(archive_resp.status(), Status::NoContent);

    // Unarchive
    let unarchive_resp = client.post(format!("{}/accounts/{}/unarchive", V2_BASE, account_id)).dispatch().await;
    assert_eq!(unarchive_resp.status(), Status::NoContent);

    // Verify status back to active via GET
    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "active");
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
    assert_eq!(decrypt_i64(body["currentBalanceEnc"].as_str().unwrap()), 50000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_adjust_balance_downward() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let account_id = common::entities::create_account(&client, "Shrink Me", 50000).await;

    let payload = json!({ "newBalance": 5000 });

    let resp = client
        .post(format!("{}/accounts/{}/adjust-balance", V2_BASE, account_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let get_resp = client.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();
    let balance = decrypt_i64(body["currentBalanceEnc"].as_str().expect("currentBalanceEnc"));
    assert_eq!(balance, 5000);
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
    assert_eq!(options.len(), 2);

    let opt_a = options
        .iter()
        .find(|o| o["id"].as_str() == Some(&id_a))
        .expect("Option Alpha not found in options");
    assert_eq!(decrypt_string(opt_a["nameEnc"].as_str().unwrap()), "Option Alpha");
    assert!(opt_a["colorEnc"].is_string());

    let opt_b = options
        .iter()
        .find(|o| o["id"].as_str() == Some(&id_b))
        .expect("Option Beta not found in options");
    assert_eq!(decrypt_string(opt_b["nameEnc"].as_str().unwrap()), "Option Beta");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_excludes_archived_accounts() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let active_id = common::entities::create_account(&client, "Still Active", 10000).await;
    let archived_id = common::entities::create_account(&client, "Now Archived", 20000).await;

    let archive_resp = client.post(format!("{}/accounts/{}/archive", V2_BASE, archived_id)).dispatch().await;
    assert_eq!(archive_resp.status(), Status::NoContent);

    let resp = client.get(format!("{}/accounts/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let options = body.as_array().expect("expected plain array");

    let ids: Vec<&str> = options.iter().map(|o| o["id"].as_str().unwrap()).collect();
    assert_eq!(options.len(), 1, "only the active account should appear");
    assert!(ids.contains(&active_id.as_str()), "active account should appear in options");
    assert!(!ids.contains(&archived_id.as_str()), "archived account should not appear in options");
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

// Regression: POST /accounts with paymentDueDay way above SMALLINT
// range must return 400 (not 500). Surfaced by schemathesis.
#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_account_numeric_overflow_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = json!({
        "accountType": "creditcard",
        "name": "Overflow",
        "color": "#ff0000",
        "currencyId": eur_id,
        "initialBalance": 0,
        "paymentDueDay": 536870911i64,
    });

    let resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    let status = resp.status();
    let body = resp.into_string().await.unwrap_or_default();
    assert_ne!(status, Status::InternalServerError, "got 500: {}", body);
}

// Regression: PUT /accounts/{id} must return 400 (not 500) when the
// caller sends a different accountType (immutable per the Postgres
// trigger reject_account_type_change) and/or an unknown currencyId
// (FK violation on currencies). Surfaced by schemathesis on the
// encryption-at-rest PR (#313).
#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_account_bad_currency_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    // Create a real account so the WHERE clause matches on UPDATE.
    let create_payload = json!({
        "accountType": "checking",
        "name": "Seed Checking",
        "color": "#ff9800",
        "currencyId": eur_id,
        "initialBalance": 100000,
    });
    let create_resp = client
        .post(format!("{}/accounts", V2_BASE))
        .header(ContentType::JSON)
        .body(create_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(create_resp.status(), Status::Created);
    let created: Value = serde_json::from_str(&create_resp.into_string().await.unwrap()).unwrap();
    let account_id = created["id"].as_str().unwrap().to_string();

    // Now replay the schemathesis payload: target the existing account but
    // swap currencyId to a random UUID so the FK fails.
    let random_currency = Uuid::new_v4();
    let payload = json!({
        "accountType": "allowance",
        "name": "",
        "color": "",
        "currencyId": random_currency,
        "initialBalance": 0,
        "spendLimit": 0,
        "nextTransferAmount": 0,
        "topUpAmount": 0,
        "topUpCycle": "weekly",
        "topUpDay": 0,
        "statementCloseDay": 0,
        "paymentDueDay": 0
    });

    let resp = client
        .put(format!("{}/accounts/{}", V2_BASE, account_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    let status = resp.status();
    let body = resp.into_string().await.unwrap_or_default();
    eprintln!("STATUS={} BODY={}", status, body);
    assert_ne!(status, Status::InternalServerError, "got 500: {}", body);
}
