//! Priority 6: Concurrent-write characterization tests.
//!
//! These tests spawn concurrent operations against the same client / database
//! to observe how the v2 transaction path behaves under parallelism. They are
//! deliberately tolerant: any result that would imply data loss (final balance
//! is NOT the sum of all applied operations) is a hard failure; everything else
//! is simply recorded as the current behavior.
//!
//! Note: because `rocket::local::asynchronous::Client` is `!Sync` and each
//! test_client() owns its own request dispatch path, we share a single client
//! via `Arc` and use its `async` dispatch (which is cancel-safe and re-entrant).

mod common;

use common::auth::{create_user_and_login, get_eur_currency_id};
use common::entities::{create_account, create_category, create_period};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::{Value, json};
use std::sync::Arc;

async fn get_json(client: &Client, url: &str) -> Value {
    let resp = client.get(url.to_string()).dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "GET {url} failed");
    serde_json::from_str(&resp.into_string().await.unwrap()).unwrap()
}

async fn get_balance(client: &Client, account_id: &str, period_id: &str) -> i64 {
    let body = get_json(client, &format!("{V2_BASE}/accounts/summary?periodId={period_id}")).await;
    body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"].as_str() == Some(account_id))
        .expect("account in summary")["currentBalance"]
        .as_i64()
        .unwrap()
}

async fn post_tx(client: &Client, payload: Value) -> Status {
    let resp = client
        .post(format!("{V2_BASE}/transactions"))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    resp.status()
}

async fn create_allowance(client: &Client, name: &str, initial: i64) -> String {
    let eur = get_eur_currency_id(client).await;
    let payload = json!({
        "type": "Allowance",
        "name": name,
        "color": "#abcdef",
        "initialBalance": initial,
        "currencyId": eur,
        "spendLimit": null
    });
    let resp = client
        .post(format!("{V2_BASE}/accounts"))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);
    serde_json::from_str::<Value>(&resp.into_string().await.unwrap()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Two concurrent transfers from the same source account
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn concurrent_transfers_from_same_source() {
    let client = Arc::new(test_client().await);
    create_user_and_login(&client).await;

    let src = create_account(&client, "Conc Src", 100_000).await;
    let dest1 = create_allowance(&client, "Conc Dest1", 0).await;
    let dest2 = create_allowance(&client, "Conc Dest2", 0).await;
    let transfer_cat = create_category(&client, "Conc Transfer", "transfer").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let c1 = Arc::clone(&client);
    let src1 = src.clone();
    let d1 = dest1.clone();
    let cat1 = transfer_cat.clone();
    let h1 = tokio::spawn(async move {
        post_tx(
            &c1,
            json!({
                "transactionType": "Transfer",
                "date": "2026-03-10",
                "description": "concurrent1",
                "amount": 10_000,
                "fromAccountId": src1,
                "categoryId": cat1,
                "vendorId": null,
                "toAccountId": d1,
            }),
        )
        .await
    });

    let c2 = Arc::clone(&client);
    let src2 = src.clone();
    let d2 = dest2.clone();
    let cat2 = transfer_cat.clone();
    let h2 = tokio::spawn(async move {
        post_tx(
            &c2,
            json!({
                "transactionType": "Transfer",
                "date": "2026-03-10",
                "description": "concurrent2",
                "amount": 15_000,
                "fromAccountId": src2,
                "categoryId": cat2,
                "vendorId": null,
                "toAccountId": d2,
            }),
        )
        .await
    });

    let s1 = h1.await.unwrap();
    let s2 = h2.await.unwrap();
    eprintln!("CHARACTERIZATION: concurrent transfers statuses: {s1} / {s2}");

    // At least one must succeed; we do not require both.
    assert!(s1 == Status::Created || s2 == Status::Created, "at least one transfer must succeed");

    // Final balances must be a consistent projection of whichever transfers
    // actually succeeded. Compute expectation from statuses.
    let mut expected_src = 100_000_i64;
    let mut expected_d1 = 0_i64;
    let mut expected_d2 = 0_i64;
    if s1 == Status::Created {
        expected_src -= 10_000;
        expected_d1 += 10_000;
    }
    if s2 == Status::Created {
        expected_src -= 15_000;
        expected_d2 += 15_000;
    }
    assert_eq!(
        get_balance(&client, &src, &period_id).await,
        expected_src,
        "source balance must reflect successful transfers"
    );
    assert_eq!(get_balance(&client, &dest1, &period_id).await, expected_d1);
    assert_eq!(get_balance(&client, &dest2, &period_id).await, expected_d2);
}

// ─────────────────────────────────────────────────────────────────────────────
// Two concurrent regular transactions on the same account
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn concurrent_transactions_same_account() {
    let client = Arc::new(test_client().await);
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Conc Acct", 100_000).await;
    let cat = create_category(&client, "Conc Cat", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let mut handles = vec![];
    for amount in [4_000, 6_000, 7_500] {
        let c = Arc::clone(&client);
        let a = acct.clone();
        let k = cat.clone();
        handles.push(tokio::spawn(async move {
            post_tx(
                &c,
                json!({
                    "transactionType": "Regular",
                    "date": "2026-03-10",
                    "description": format!("conc {amount}"),
                    "amount": amount,
                    "fromAccountId": a,
                    "categoryId": k,
                    "vendorId": null,
                }),
            )
            .await
        }));
    }

    let mut total_succeeded = 0_i64;
    let amounts = [4_000_i64, 6_000, 7_500];
    for (h, a) in handles.into_iter().zip(amounts) {
        let status = h.await.unwrap();
        if status == Status::Created {
            total_succeeded += a;
        } else {
            eprintln!("CHARACTERIZATION: concurrent tx failed with {status}");
        }
    }
    assert!(total_succeeded > 0, "at least one concurrent tx must succeed");
    assert_eq!(
        get_balance(&client, &acct, &period_id).await,
        100_000 - total_succeeded,
        "final balance must reflect all successful txs"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Concurrent create + list read
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn concurrent_create_and_read_no_torn_state() {
    let client = Arc::new(test_client().await);
    create_user_and_login(&client).await;

    let acct = create_account(&client, "CR Acct", 100_000).await;
    let cat = create_category(&client, "CR Cat", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let c1 = Arc::clone(&client);
    let a1 = acct.clone();
    let k1 = cat.clone();
    let writer = tokio::spawn(async move {
        post_tx(
            &c1,
            json!({
                "transactionType": "Regular",
                "date": "2026-03-10",
                "description": "concurrent-read",
                "amount": 3_333,
                "fromAccountId": a1,
                "categoryId": k1,
                "vendorId": null,
            }),
        )
        .await
    });

    let c2 = Arc::clone(&client);
    let p = period_id.clone();
    let reader = tokio::spawn(async move {
        // Read should never fail or return malformed data.
        get_json(&c2, &format!("{V2_BASE}/transactions?periodId={p}")).await
    });

    let w_status = writer.await.unwrap();
    let list = reader.await.unwrap();
    assert_eq!(w_status, Status::Created);
    assert!(list["data"].is_array(), "list must return a valid array during concurrent write");

    // Final state: read after join, must contain exactly the new tx.
    let final_list = get_json(&client, &format!("{V2_BASE}/transactions?periodId={period_id}")).await;
    let txs = final_list["data"].as_array().unwrap();
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0]["amount"], 3_333);
}

// ─────────────────────────────────────────────────────────────────────────────
// Concurrent update + delete on the same transaction
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn concurrent_update_and_delete_same_tx() {
    let client = Arc::new(test_client().await);
    create_user_and_login(&client).await;

    let acct = create_account(&client, "UD Acct", 100_000).await;
    let cat = create_category(&client, "UD Cat", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let tx_id = common::entities::create_transaction(&client, &acct, &cat, 5_000, "2026-03-10").await;

    let c1 = Arc::clone(&client);
    let tx1 = tx_id.clone();
    let a1 = acct.clone();
    let k1 = cat.clone();
    let updater = tokio::spawn(async move {
        c1.put(format!("{V2_BASE}/transactions/{tx1}"))
            .header(ContentType::JSON)
            .body(
                json!({
                    "transactionType": "Regular",
                    "date": "2026-03-10",
                    "description": "updated",
                    "amount": 9_999,
                    "fromAccountId": a1,
                    "categoryId": k1,
                    "vendorId": null,
                })
                .to_string(),
            )
            .dispatch()
            .await
            .status()
    });

    let c2 = Arc::clone(&client);
    let tx2 = tx_id.clone();
    let deleter = tokio::spawn(async move { c2.delete(format!("{V2_BASE}/transactions/{tx2}")).dispatch().await.status() });

    let upd_status = updater.await.unwrap();
    let del_status = deleter.await.unwrap();
    eprintln!("CHARACTERIZATION: concurrent update+delete → update={upd_status}, delete={del_status}");

    // Outcomes that are acceptable:
    //  a) both report success (racy — the delete wins, but update may have
    //     landed first and delete then removed it)
    //  b) delete succeeds, update returns 404/409
    //  c) update succeeds, delete returns 404
    assert!(upd_status == Status::Ok || upd_status == Status::NotFound || upd_status == Status::Conflict);
    assert!(del_status == Status::NoContent || del_status == Status::NotFound || del_status == Status::Conflict);

    // Whatever happened, the period's tx list and the account balance must
    // agree with each other.
    let list = get_json(&client, &format!("{V2_BASE}/transactions?periodId={period_id}")).await;
    let txs = list["data"].as_array().unwrap();
    let final_balance = get_balance(&client, &acct, &period_id).await;

    if txs.is_empty() {
        // Tx was deleted → balance fully restored
        assert_eq!(final_balance, 100_000);
    } else {
        // Tx still exists → balance reflects whatever amount survived
        assert_eq!(txs.len(), 1);
        let amount = txs[0]["amount"].as_i64().unwrap();
        assert_eq!(final_balance, 100_000 - amount);
    }
}
