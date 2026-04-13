//! Priority 4: Cascading-delete reversal characterization.
//!
//! Document what v2 actually does when the parent of a transaction is deleted.
//! These tests observe behavior rather than enforce a preferred policy — the
//! ledger refactor may change the rules, at which point these tests will
//! surface exactly what moved.

mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_period, create_transaction, create_transaction_with_vendor, create_vendor};
use common::{V2_BASE, test_client};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use serde_json::Value;

async fn get_status_json(client: &Client, url: &str) -> (Status, Option<Value>) {
    let resp = client.get(url.to_string()).dispatch().await;
    let status = resp.status();
    let body = resp.into_string().await.and_then(|s| serde_json::from_str(&s).ok());
    (status, body)
}

async fn get_json(client: &Client, url: &str) -> Value {
    let (status, body) = get_status_json(client, url).await;
    assert_eq!(status, Status::Ok, "GET {url} failed with {status}");
    body.expect("body")
}

async fn delete_and_expect(client: &Client, url: &str) -> Status {
    client.delete(url.to_string()).dispatch().await.status()
}

async fn list_period_txs(client: &Client, period_id: &str) -> Vec<Value> {
    let body = get_json(client, &format!("{V2_BASE}/transactions?periodId={period_id}")).await;
    body["data"].as_array().unwrap().clone()
}

async fn account_balance(client: &Client, account_id: &str, period_id: &str) -> i64 {
    let body = get_json(client, &format!("{V2_BASE}/accounts/summary?periodId={period_id}")).await;
    body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"].as_str() == Some(account_id))
        .expect("account in summary")["currentBalance"]
        .as_i64()
        .expect("currentBalance")
}

// ─────────────────────────────────────────────────────────────────────────────
// Delete account that has transactions
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn delete_account_with_transactions_characterization() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "DelAcct", 100_000).await;
    let cat = create_category(&client, "DelAcct Cat", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    for i in 0..5 {
        create_transaction(&client, &acct, &cat, 1_000 + i * 100, "2026-03-10").await;
    }
    let before = list_period_txs(&client, &period_id).await;
    assert_eq!(before.len(), 5);

    // Pre-delete balance snapshot
    let pre_balance = account_balance(&client, &acct, &period_id).await;
    assert_eq!(pre_balance, 100_000 - (1_000 + 1_100 + 1_200 + 1_300 + 1_400));

    // Characterization: attempt delete and see what happens.
    let status = delete_and_expect(&client, &format!("{V2_BASE}/accounts/{acct}")).await;
    eprintln!("CHARACTERIZATION: delete account with 5 txs → status {status}");

    if status == Status::NoContent {
        // Cascading delete path: transactions list endpoint must still work.
        let (st, _) = get_status_json(&client, &format!("{V2_BASE}/transactions?periodId={period_id}")).await;
        assert_eq!(st, Status::Ok, "transactions list must still work after account deletion");
    } else {
        // Delete blocked — account and txs must still exist with balance unchanged.
        assert!(
            status == Status::Conflict || status == Status::BadRequest || status == Status::UnprocessableEntity,
            "unexpected status: {status}"
        );
        let post_balance = account_balance(&client, &acct, &period_id).await;
        assert_eq!(post_balance, pre_balance, "balance must not change when delete is blocked");
        assert_eq!(list_period_txs(&client, &period_id).await.len(), 5);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Delete period that contains transactions
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn delete_period_with_transactions_characterization() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "DelPer Acct", 100_000).await;
    let cat = create_category(&client, "DelPer Cat", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let tx_id = create_transaction(&client, &acct, &cat, 4_000, "2026-03-10").await;

    let pre_balance = account_balance(&client, &acct, &period_id).await;
    assert_eq!(pre_balance, 96_000);

    let status = delete_and_expect(&client, &format!("{V2_BASE}/periods/{period_id}")).await;
    eprintln!("CHARACTERIZATION: delete period containing 1 tx → status {status}");

    if status == Status::NoContent {
        // Period is gone; create another overlapping period and check what's visible.
        let peek = create_period(&client, "2026-03-01", "2026-03-31").await;
        let txs = list_period_txs(&client, &peek).await;
        eprintln!("CHARACTERIZATION: after period delete, new overlapping period sees {} txs", txs.len());

        // Account balance invariant must still hold, using the new period scope.
        let post_balance = account_balance(&client, &acct, &peek).await;
        assert_eq!(post_balance, pre_balance, "account balance must not change when a period is deleted");
    } else {
        // Delete blocked — everything should be intact.
        let txs = list_period_txs(&client, &period_id).await;
        assert!(txs.iter().any(|t| t["id"] == tx_id.as_str()));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Delete category that has transactions → v2 made category_id nullable
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn delete_category_with_transactions_characterization() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "DelCat Acct", 100_000).await;
    let cat = create_category(&client, "DelCat Cat", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let tx_id = create_transaction(&client, &acct, &cat, 2_500, "2026-03-10").await;

    let pre_balance = account_balance(&client, &acct, &period_id).await;

    let status = delete_and_expect(&client, &format!("{V2_BASE}/categories/{cat}")).await;
    eprintln!("CHARACTERIZATION: delete category with 1 tx → status {status}");

    if status == Status::NoContent {
        // Txs should remain, but with NULL category (since v2 made it nullable).
        let txs = list_period_txs(&client, &period_id).await;
        let found = txs.iter().find(|t| t["id"] == tx_id.as_str());
        if let Some(t) = found {
            assert!(t["category"].is_null() || t["categoryId"].is_null(), "category should be null after delete");
        }
        let post_balance = account_balance(&client, &acct, &period_id).await;
        assert_eq!(post_balance, pre_balance);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Delete vendor that has transactions
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn delete_vendor_with_transactions_is_blocked() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "DelVend Acct", 100_000).await;
    let cat = create_category(&client, "DelVend Cat", "expense").await;
    let vendor = create_vendor(&client, "DelVendor").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let tx_id = create_transaction_with_vendor(&client, &acct, &cat, 3_200, "2026-03-11", &vendor).await;

    let pre_balance = account_balance(&client, &acct, &period_id).await;

    // Vendor delete must be blocked when transactions reference it — mirrors
    // the behavior of accounts and categories. The user must archive instead.
    let status = delete_and_expect(&client, &format!("{V2_BASE}/vendors/{vendor}")).await;
    assert_eq!(status, Status::BadRequest, "delete should be rejected when transactions exist");

    // Transaction must still exist
    let txs = list_period_txs(&client, &period_id).await;
    let found = txs.iter().find(|t| t["id"] == tx_id.as_str());
    assert!(found.is_some(), "transaction should still exist after blocked vendor delete");

    // Balance must be unchanged
    let post_balance = account_balance(&client, &acct, &period_id).await;
    assert_eq!(post_balance, pre_balance, "balance should be unchanged");
}
