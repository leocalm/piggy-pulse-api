mod common;

use common::test_client;
use rocket::http::Status;
use serde_json::Value;

// ── List currencies ──────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_currencies_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies", common::V2_BASE)).dispatch().await;

    // Public endpoint — must work without authentication
    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_currencies_response_shape() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let arr = body.as_array().expect("expected array");
    assert!(!arr.is_empty(), "expected at least one currency");

    let first = &arr[0];
    common::assertions::assert_uuid(&first["id"]);
    assert!(first["name"].is_string());
    assert!(first["symbol"].is_string());
    assert!(first["code"].is_string());
    assert!(first["decimalPlaces"].is_number());
    assert!(first["symbolPosition"].is_string());
}

// ── Get currency by code ─────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_currency_by_code() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies/EUR", common::V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["code"], "EUR");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_currency_invalid_code() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies/INVALID", common::V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}
