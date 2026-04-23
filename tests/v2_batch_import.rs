mod common;

use common::auth::create_user_and_login;
use common::crypto::{decrypt_i64, decrypt_string};
use common::entities::{create_account, create_category};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /transactions/batch
// Returns Vec<EncryptedTransactionResponse> — amount/description are encrypted.
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
            "transactionType": "Regular",
            "description": "First",
            "amount": 1000,
            "date": "2026-03-01",
            "fromAccountId": account_id,
            "categoryId": cat_id
        },
        {
            "transactionType": "Regular",
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

    // Encrypted response fields
    assert_eq!(decrypt_string(arr[0]["descriptionEnc"].as_str().unwrap()), "First");
    assert_eq!(decrypt_i64(arr[0]["amountEnc"].as_str().unwrap()), 1000);
    assert_eq!(decrypt_string(arr[1]["descriptionEnc"].as_str().unwrap()), "Second");
    assert_eq!(decrypt_i64(arr[1]["amountEnc"].as_str().unwrap()), 2000);
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
