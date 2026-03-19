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
async fn test_list_currencies_is_plain_array() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let arr = body.as_array().expect("expected plain array, not wrapped object");
    assert!(!arr.is_empty(), "expected at least one currency");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_currencies_each_has_all_fields() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let arr = body.as_array().unwrap();

    for currency in arr {
        common::assertions::assert_uuid(&currency["id"]);
        assert!(currency["name"].is_string(), "name must be string");
        assert!(currency["symbol"].is_string(), "symbol must be string");
        assert!(currency["code"].is_string(), "code must be string");
        assert!(currency["decimalPlaces"].is_number(), "decimalPlaces must be number");
        let sp = currency["symbolPosition"].as_str().expect("symbolPosition must be string");
        assert!(sp == "before" || sp == "after", "symbolPosition must be before or after, got {}", sp);
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_currencies_contains_eur_with_correct_values() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let arr = body.as_array().unwrap();

    let eur = arr.iter().find(|c| c["code"] == "EUR").expect("EUR must be present in currencies list");

    assert_eq!(eur["name"], "Euro");
    assert_eq!(eur["symbol"], "\u{20ac}"); // euro sign
    assert_eq!(eur["code"], "EUR");
    assert_eq!(eur["decimalPlaces"], 2);
    assert_eq!(eur["symbolPosition"], "before");
    common::assertions::assert_uuid(&eur["id"]);
}

// ── Get currency by code ─────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_currency_eur_returns_correct_fields() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies/EUR", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["code"], "EUR");
    assert_eq!(body["name"], "Euro");
    assert_eq!(body["symbol"], "\u{20ac}");
    assert_eq!(body["decimalPlaces"], 2);
    assert_eq!(body["symbolPosition"], "before");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_currency_unknown_code_returns_404() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies/XYZ", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_currency_is_public_endpoint() {
    // No authentication — should still return 200
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies/EUR", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_currencies_decimal_places_are_positive() {
    let client = test_client().await;

    let resp = client.get(format!("{}/currencies", common::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let arr = body.as_array().unwrap();

    for currency in arr {
        let dp = currency["decimalPlaces"].as_i64().expect("decimalPlaces must be number");
        assert!(dp >= 0, "decimalPlaces must be non-negative, got {} for {}", dp, currency["code"]);
    }
}
