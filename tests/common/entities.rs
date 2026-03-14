use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;

use super::auth::get_eur_currency_id;

/// Creates an account via V2 POST /accounts. Returns the account ID.
pub async fn create_account(client: &Client, name: &str, balance: i64) -> String {
    let eur_id = get_eur_currency_id(client).await;
    let payload = serde_json::json!({
        "type": "Checking",
        "name": name,
        "color": "#1a2b3c",
        "initialBalance": balance,
        "currencyId": eur_id,
        "spendLimit": null
    });

    let resp = client
        .post(format!("{}/accounts", super::V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_account failed");

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("account body")).expect("valid json");
    body["id"].as_str().expect("account id").to_string()
}

/// Creates a category via V2 POST /categories. Returns the category ID.
pub async fn create_category(client: &Client, name: &str, category_type: &str) -> String {
    let payload = serde_json::json!({
        "name": name,
        "type": category_type,
        "icon": "🛒",
        "color": "#123456",
        "description": null,
        "parentId": null
    });

    let resp = client
        .post(format!("{}/categories", super::V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_category failed");

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("category body")).expect("valid json");
    body["id"].as_str().expect("category id").to_string()
}

/// Creates a vendor via V2 POST /vendors. Returns the vendor ID.
pub async fn create_vendor(client: &Client, name: &str) -> String {
    let payload = serde_json::json!({
        "name": name,
        "description": null
    });

    let resp = client
        .post(format!("{}/vendors", super::V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_vendor failed");

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("vendor body")).expect("valid json");
    body["id"].as_str().expect("vendor id").to_string()
}

/// Creates a period via V2 POST /periods. Returns the period ID.
pub async fn create_period(client: &Client, start: &str, end: &str) -> String {
    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": start,
        "name": format!("Period {}", uuid::Uuid::new_v4()),
        "manualEndDate": end
    });

    let resp = client
        .post(format!("{}/periods", super::V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_period failed");

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("period body")).expect("valid json");
    body["id"].as_str().expect("period id").to_string()
}

/// Creates a transaction via V2 POST /transactions. Returns the transaction ID.
pub async fn create_transaction(client: &Client, from_account_id: &str, category_id: &str, amount: i64, date: &str) -> String {
    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": date,
        "description": "Test transaction",
        "amount": amount,
        "fromAccountId": from_account_id,
        "categoryId": category_id,
        "vendorId": null
    });

    let resp = client
        .post(format!("{}/transactions", super::V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_transaction failed");

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("transaction body")).expect("valid json");
    body["id"].as_str().expect("transaction id").to_string()
}

/// Creates an overlay via V2 POST /overlays. Returns the overlay ID.
pub async fn create_overlay(client: &Client, name: &str, start: &str, end: &str) -> String {
    let payload = serde_json::json!({
        "name": name,
        "icon": null,
        "startDate": start,
        "endDate": end,
        "inclusionMode": "manual",
        "totalCapAmount": null,
        "categoryCaps": [],
        "rules": null
    });

    let resp = client
        .post(format!("{}/overlays", super::V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_overlay failed");

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("overlay body")).expect("valid json");
    body["id"].as_str().expect("overlay id").to_string()
}
