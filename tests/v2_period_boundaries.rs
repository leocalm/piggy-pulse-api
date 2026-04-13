//! Priority 3: Period boundary edge cases.
//!
//! Locks in current v2 behavior around date inclusivity of periods: start_date
//! and end_date are both inclusive, one-day periods are valid, and moving a
//! transaction across a period boundary shifts its membership.

mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_period, create_transaction};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::{Value, json};

async fn get_json(client: &Client, url: &str) -> Value {
    let resp = client.get(url.to_string()).dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "GET {url} failed");
    serde_json::from_str(&resp.into_string().await.unwrap()).unwrap()
}

async fn period_spent(client: &Client, period_id: &str) -> i64 {
    get_json(client, &format!("{V2_BASE}/dashboard/current-period?periodId={period_id}")).await["spent"]
        .as_i64()
        .unwrap()
}

async fn period_tx_ids(client: &Client, period_id: &str) -> Vec<String> {
    let body = get_json(client, &format!("{V2_BASE}/transactions?periodId={period_id}")).await;
    body["data"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap().to_string()).collect()
}

async fn put_transaction(client: &Client, tx_id: &str, payload: Value) {
    let resp = client
        .put(format!("{V2_BASE}/transactions/{tx_id}"))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok, "PUT tx failed");
}

async fn put_period(client: &Client, period_id: &str, start: &str, end: &str, name: &str) {
    let resp = client
        .put(format!("{V2_BASE}/periods/{period_id}"))
        .header(ContentType::JSON)
        .body(
            json!({
                "periodType": "ManualEndDate",
                "startDate": start,
                "name": name,
                "manualEndDate": end,
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok, "PUT period failed");
}

// ─────────────────────────────────────────────────────────────────────────────
// Inclusive boundaries
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn tx_on_start_date_included() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Bound A", 100_000).await;
    let cat = create_category(&client, "Bound Cat A", "expense").await;
    let period_id = create_period(&client, "2026-06-01", "2026-06-30").await;

    create_transaction(&client, &acct, &cat, 1_000, "2026-06-01").await;
    assert_eq!(period_spent(&client, &period_id).await, 1_000);
    assert_eq!(period_tx_ids(&client, &period_id).await.len(), 1);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn tx_on_end_date_included() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Bound B", 100_000).await;
    let cat = create_category(&client, "Bound Cat B", "expense").await;
    let period_id = create_period(&client, "2026-06-01", "2026-06-30").await;

    create_transaction(&client, &acct, &cat, 2_000, "2026-06-30").await;
    assert_eq!(period_spent(&client, &period_id).await, 2_000);
    assert_eq!(period_tx_ids(&client, &period_id).await.len(), 1);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn tx_day_after_end_date_not_in_period_but_in_next() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Bound C", 100_000).await;
    let cat = create_category(&client, "Bound Cat C", "expense").await;
    let june = create_period(&client, "2026-06-01", "2026-06-30").await;
    let july = create_period(&client, "2026-07-01", "2026-07-31").await;

    create_transaction(&client, &acct, &cat, 3_000, "2026-07-01").await;
    assert_eq!(period_spent(&client, &june).await, 0, "June should be empty");
    assert_eq!(period_spent(&client, &july).await, 3_000, "July should contain the tx");
    assert_eq!(period_tx_ids(&client, &june).await.len(), 0);
    assert_eq!(period_tx_ids(&client, &july).await.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Cross-boundary move
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn move_tx_across_period_boundary() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "CB Acct", 100_000).await;
    let cat = create_category(&client, "CB Cat", "expense").await;
    let jan = create_period(&client, "2026-01-01", "2026-01-31").await;
    let feb = create_period(&client, "2026-02-01", "2026-02-28").await;

    let tx_id = create_transaction(&client, &acct, &cat, 4_500, "2026-01-31").await;
    assert_eq!(period_spent(&client, &jan).await, 4_500);
    assert_eq!(period_spent(&client, &feb).await, 0);

    // Move to Feb 1
    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-02-01",
            "description": "Test transaction",
            "amount": 4_500,
            "fromAccountId": acct,
            "categoryId": cat,
            "vendorId": null
        }),
    )
    .await;

    assert_eq!(period_spent(&client, &jan).await, 0, "Jan should no longer include the tx");
    assert_eq!(period_spent(&client, &feb).await, 4_500, "Feb should now include the tx");
}

// ─────────────────────────────────────────────────────────────────────────────
// One-day period
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn single_day_period_rejected_by_api() {
    // CHARACTERIZATION: v2 rejects periods where manualEndDate == startDate
    // with "manualEndDate must be after startDate". The smallest valid period
    // is 2 days. The ledger refactor should revisit whether 1-day periods
    // should be supported.
    use rocket::http::ContentType;
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .post(format!("{V2_BASE}/periods"))
        .header(ContentType::JSON)
        .body(
            json!({
                "periodType": "ManualEndDate",
                "startDate": "2026-06-15",
                "name": "one day",
                "manualEndDate": "2026-06-15",
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest, "1-day period is currently rejected");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn two_day_period_is_smallest_valid() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "2D Acct", 100_000).await;
    let cat = create_category(&client, "2D Cat", "expense").await;
    let two_day = create_period(&client, "2026-06-15", "2026-06-16").await;
    create_transaction(&client, &acct, &cat, 777, "2026-06-15").await;
    assert_eq!(period_spent(&client, &two_day).await, 777);
    let cp = get_json(&client, &format!("{V2_BASE}/dashboard/current-period?periodId={two_day}")).await;
    assert_eq!(cp["daysInPeriod"], 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// Period range changes
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn extend_period_start_date_includes_new_txs() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Ext Acct", 100_000).await;
    let cat = create_category(&client, "Ext Cat", "expense").await;
    let period_id = create_period(&client, "2026-06-10", "2026-06-30").await;

    // Tx on June 5 (before period start)
    create_transaction(&client, &acct, &cat, 5_000, "2026-06-05").await;
    assert_eq!(period_spent(&client, &period_id).await, 0);

    // Extend period back to June 1
    put_period(&client, &period_id, "2026-06-01", "2026-06-30", "Extended").await;

    assert_eq!(period_spent(&client, &period_id).await, 5_000, "tx should be pulled in by extended range");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn shrink_period_end_date_excludes_txs() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Shr Acct", 100_000).await;
    let cat = create_category(&client, "Shr Cat", "expense").await;
    let period_id = create_period(&client, "2026-06-01", "2026-06-30").await;

    create_transaction(&client, &acct, &cat, 6_000, "2026-06-25").await;
    assert_eq!(period_spent(&client, &period_id).await, 6_000);

    // Shrink to end at June 20 → tx falls outside
    put_period(&client, &period_id, "2026-06-01", "2026-06-20", "Shrunk").await;
    assert_eq!(period_spent(&client, &period_id).await, 0, "tx should drop out when end date shrinks");
}
