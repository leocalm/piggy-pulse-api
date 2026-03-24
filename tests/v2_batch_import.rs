mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_transaction};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /transactions/batch
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_batch_create_transactions_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = create_account(&client, "Batch Bank", 500_000).await;
    let cat_id = create_category(&client, "Batch Cat", "expense").await;

    let payload = serde_json::json!([
        {
            "transactionType": "regular",
            "description": "First",
            "amount": 1000,
            "date": "2026-03-01",
            "fromAccountId": account_id,
            "categoryId": cat_id
        },
        {
            "transactionType": "regular",
            "description": "Second",
            "amount": 2000,
            "date": "2026-03-02",
            "fromAccountId": account_id,
            "categoryId": cat_id
        }
    ]);

    let resp = client
        .post(format!("{}/transactions/batch", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let arr = body.as_array().expect("response is array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["description"], "First");
    assert_eq!(arr[0]["amount"], 1000);
    assert_eq!(arr[1]["description"], "Second");
    assert_eq!(arr[1]["amount"], 2000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_batch_create_transactions_empty_array() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .post(format!("{}/transactions/batch", V2_BASE))
        .header(ContentType::JSON)
        .body("[]")
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_batch_create_transactions_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client
        .post(format!("{}/transactions/batch", V2_BASE))
        .header(ContentType::JSON)
        .body("[]")
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /settings/import/data
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_import_data_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create some data and export it
    let _account_id = create_account(&client, "Import Source Account", 100_000).await;
    let _cat_id = create_category(&client, "Import Source Cat", "expense").await;

    let export_resp = client.get(format!("{}/settings/export/data", V2_BASE)).dispatch().await;
    assert_eq!(export_resp.status(), Status::Ok);
    let export_data: Value = serde_json::from_str(&export_resp.into_string().await.unwrap()).unwrap();

    // Import the exported data back
    let import_resp = client
        .post(format!("{}/settings/import/data", V2_BASE))
        .header(ContentType::JSON)
        .body(export_data.to_string())
        .dispatch()
        .await;

    assert_eq!(import_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&import_resp.into_string().await.unwrap()).unwrap();

    // Verify response includes counts
    assert!(body["imported"]["accounts"].as_i64().unwrap() >= 1);
    assert!(body["imported"]["categories"].as_i64().unwrap() >= 1);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_import_data_with_transactions() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = create_account(&client, "Import Tx Account", 200_000).await;
    let cat_id = create_category(&client, "Import Tx Cat", "expense").await;
    create_transaction(&client, &account_id, &cat_id, 5000, "2026-03-10").await;

    let export_resp = client.get(format!("{}/settings/export/data", V2_BASE)).dispatch().await;
    assert_eq!(export_resp.status(), Status::Ok);
    let export_data: Value = serde_json::from_str(&export_resp.into_string().await.unwrap()).unwrap();

    let import_resp = client
        .post(format!("{}/settings/import/data", V2_BASE))
        .header(ContentType::JSON)
        .body(export_data.to_string())
        .dispatch()
        .await;

    assert_eq!(import_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&import_resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["imported"]["transactions"], 1);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_import_data_empty_payload() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({ "accounts": [], "categories": [], "transactions": [] });

    let resp = client
        .post(format!("{}/settings/import/data", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["imported"]["accounts"], 0);
    assert_eq!(body["imported"]["categories"], 0);
    assert_eq!(body["imported"]["transactions"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_import_data_unauthenticated_returns_401() {
    let client = test_client().await;

    let payload = serde_json::json!({ "accounts": [], "categories": [], "transactions": [] });

    let resp = client
        .post(format!("{}/settings/import/data", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
