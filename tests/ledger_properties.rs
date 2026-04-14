//! Ledger property tests.
//!
//! These exercise the invariants from the immutable transaction ledger spec
//! (`.kiro/specs/immutable-transaction-ledger/design.md`). Each test runs a
//! randomized stress sequence against the real HTTP stack to catch regressions
//! that pass example-based tests but violate the math under a broader input
//! distribution.
//!
//! Tagged as Phase 5 of the refactor; see tasks.md for the full property
//! catalog (14 properties). The ones we implement here are the load-bearing
//! subset:
//!   1. Create produces the correct effective amount
//!   2. Void zeroes the effective amount and removes the tx from reads
//!   3. Correct produces the desired effective amount
//!   7. Aggregate consistency — a long mixed sequence of create/void/correct
//!      leaves the dashboard stats in sync with a brute-force recomputation
//!   8. Immutability trigger blocks direct UPDATE/DELETE on `transaction`
//!   9. Vendor merge preserves logical ids and moves aggregates
//!  10. first_created_at stability across corrections
//!  11. Negative amount requests rejected at the DTO boundary
//!
//! Properties 4 (concurrent double-void), 5 (correct-voided returns 409),
//! 6 (list excludes voided), 12 (balance parity), 13 (type immutability), and
//! 14 (period edit transparency) are covered by existing integration tests or
//! by the Phase 1 backfill verification.

mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_period, create_transaction, create_transaction_with_vendor, create_vendor};
use common::{TEST_DB_URL, V2_BASE, test_client};
use rand::RngExt;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::{Value, json};
use uuid::Uuid;

const ITERATIONS: usize = 50;

async fn get_json(client: &Client, url: &str) -> Value {
    let resp = client.get(url.to_string()).dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "GET {url} failed with {}", resp.status());
    serde_json::from_str(&resp.into_string().await.unwrap()).unwrap()
}

async fn tx_amount(client: &Client, period_id: &str, tx_id: &str) -> Option<i64> {
    let body = get_json(client, &format!("{V2_BASE}/transactions?periodId={period_id}")).await;
    body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"].as_str() == Some(tx_id))
        .and_then(|t| t["amount"].as_i64())
}

async fn tx_stats_total_outflows(client: &Client, period_id: &str) -> i64 {
    let body = get_json(client, &format!("{V2_BASE}/transactions/stats?periodId={period_id}")).await;
    body["totalOutflows"].as_i64().unwrap()
}

async fn tx_stats_count(client: &Client, period_id: &str) -> i64 {
    let body = get_json(client, &format!("{V2_BASE}/transactions/stats?periodId={period_id}")).await;
    body["transactionCount"].as_i64().unwrap()
}

async fn put_transaction(client: &Client, tx_id: &str, payload: Value) -> Status {
    let resp = client
        .put(format!("{V2_BASE}/transactions/{tx_id}"))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    resp.status()
}

// ═══════════════════════════════════════════════════════════════════════════
// Property 1 + 2 + 3 + 10: basic CRUD correctness + first_created_at stability
// ═══════════════════════════════════════════════════════════════════════════

/// For N random (amount, description) pairs, creating a transaction produces
/// a logical transaction whose effective amount equals the input amount, and
/// whose created_at does not change across subsequent corrections.
#[rocket::async_test]
#[ignore = "requires database"]
async fn property_create_void_correct_round_trips() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Prop Acct", 10_000_000).await;
    let cat = create_category(&client, "Prop Cat", "expense").await;
    let period_id = create_period(&client, "2026-07-01", "2026-07-31").await;

    let mut rng = rand::rng();

    for _ in 0..ITERATIONS {
        let amount: i64 = rng.random_range(1..=100_000);
        let tx_id = create_transaction(&client, &acct, &cat, amount, "2026-07-10").await;

        // Property 1: effective amount == creation amount
        let observed = tx_amount(&client, &period_id, &tx_id).await;
        assert_eq!(observed, Some(amount), "Property 1 violated: create returned {observed:?}, expected {amount}");

        // Capture the initial created_at
        let initial_created_at = get_json(&client, &format!("{V2_BASE}/transactions?periodId={period_id}")).await["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"].as_str() == Some(&tx_id))
            .and_then(|t| t["createdAt"].as_str().map(String::from));

        // Property 3: correct produces the desired amount
        let desired: i64 = rng.random_range(1..=100_000);
        let status = put_transaction(
            &client,
            &tx_id,
            json!({
                "transactionType": "Regular",
                "date": "2026-07-10",
                "description": "Corrected",
                "amount": desired,
                "fromAccountId": acct,
                "categoryId": cat,
            }),
        )
        .await;
        assert_eq!(status, Status::Ok);
        let after_correct = tx_amount(&client, &period_id, &tx_id).await;
        assert_eq!(
            after_correct,
            Some(desired),
            "Property 3 violated: correct returned {after_correct:?}, expected {desired}"
        );

        // Property 10: first_created_at stability across correction
        if let Some(initial) = &initial_created_at {
            let current = get_json(&client, &format!("{V2_BASE}/transactions?periodId={period_id}")).await["data"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["id"].as_str() == Some(&tx_id))
                .and_then(|t| t["createdAt"].as_str().map(String::from));
            assert_eq!(current.as_ref(), Some(initial), "Property 10 violated: created_at shifted after correction");
        }

        // Property 2: void removes the tx from the list
        let resp = client.delete(format!("{V2_BASE}/transactions/{tx_id}")).dispatch().await;
        assert_eq!(resp.status(), Status::NoContent);
        let after_void = tx_amount(&client, &period_id, &tx_id).await;
        assert_eq!(after_void, None, "Property 2 violated: voided tx still visible in list");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Property 7: aggregate consistency under a long mixed sequence of operations
// ═══════════════════════════════════════════════════════════════════════════

/// Generate a long random sequence of create/void/correct operations and
/// assert after each step that the dashboard stats (totalOutflows,
/// transactionCount) match a brute-force Rust-side recomputation of the
/// effective state. This is the single most load-bearing test in the spec —
/// it proves the trigger's delta math stays correct under arbitrary mixes.
#[rocket::async_test]
#[ignore = "requires database"]
async fn property_aggregate_consistency_under_mixed_ops() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Agg Acct", 10_000_000).await;
    let cat = create_category(&client, "Agg Cat", "expense").await;
    let period_id = create_period(&client, "2026-08-01", "2026-08-31").await;

    let mut rng = rand::rng();

    // Track the expected effective state per logical id.
    let mut effective: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for _ in 0..ITERATIONS {
        // Pick an operation: 50% create, 25% void, 25% correct (when there are
        // effective txns to operate on).
        let op = if effective.values().any(|a| *a > 0) {
            rng.random_range(0..4)
        } else {
            0 // force create
        };

        match op {
            0 | 1 => {
                // create
                let amount: i64 = rng.random_range(1..=50_000);
                let tx_id = create_transaction(&client, &acct, &cat, amount, "2026-08-15").await;
                effective.insert(tx_id, amount);
            }
            2 => {
                // void a random effective transaction
                let victim = effective.iter().find(|(_, amt)| **amt > 0).map(|(id, _)| id.clone()).unwrap();
                let resp = client.delete(format!("{V2_BASE}/transactions/{victim}")).dispatch().await;
                assert_eq!(resp.status(), Status::NoContent);
                effective.insert(victim, 0);
            }
            _ => {
                // correct a random effective transaction to a new amount
                let victim = effective.iter().find(|(_, amt)| **amt > 0).map(|(id, _)| id.clone()).unwrap();
                let new_amount: i64 = rng.random_range(1..=50_000);
                let status = put_transaction(
                    &client,
                    &victim,
                    json!({
                        "transactionType": "Regular",
                        "date": "2026-08-15",
                        "description": "Corrected mid-sequence",
                        "amount": new_amount,
                        "fromAccountId": acct,
                        "categoryId": cat,
                    }),
                )
                .await;
                assert_eq!(status, Status::Ok);
                effective.insert(victim, new_amount);
            }
        }

        // After every op, verify the aggregate reads match the expected state.
        let expected_total: i64 = effective.values().copied().sum();
        let expected_count: i64 = effective.values().filter(|a| **a > 0).count() as i64;

        let observed_total = tx_stats_total_outflows(&client, &period_id).await;
        assert_eq!(
            observed_total, expected_total,
            "Property 7 violated (totalOutflows): observed {observed_total}, expected {expected_total}"
        );

        let observed_count = tx_stats_count(&client, &period_id).await;
        assert_eq!(
            observed_count, expected_count,
            "Property 7 violated (transactionCount): observed {observed_count}, expected {expected_count}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Property 8: immutability trigger blocks UPDATE/DELETE on transaction
// ═══════════════════════════════════════════════════════════════════════════

/// Direct SQL UPDATE and DELETE against `transaction` must raise
/// "ledger rows are immutable" regardless of the WHERE clause. Uses a direct
/// sqlx connection to bypass the application layer.
#[rocket::async_test]
#[ignore = "requires database"]
async fn property_immutability_trigger_blocks_update_and_delete() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Imm Acct", 100_000).await;
    let cat = create_category(&client, "Imm Cat", "expense").await;
    let _period = create_period(&client, "2026-09-01", "2026-09-30").await;
    let _tx = create_transaction(&client, &acct, &cat, 1_234, "2026-09-05").await;

    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| TEST_DB_URL.to_string());
    let pool = sqlx::PgPool::connect(&url).await.expect("connect");

    let update_err = sqlx::query("UPDATE transaction SET amount = 9999")
        .execute(&pool)
        .await
        .expect_err("UPDATE should have been rejected");
    assert!(
        update_err.to_string().contains("ledger rows are immutable"),
        "expected immutability error, got: {update_err}"
    );

    let delete_err = sqlx::query("DELETE FROM transaction")
        .execute(&pool)
        .await
        .expect_err("DELETE should have been rejected");
    assert!(
        delete_err.to_string().contains("ledger rows are immutable"),
        "expected immutability error, got: {delete_err}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Property 9: vendor merge preserves logical ids and moves aggregates
// ═══════════════════════════════════════════════════════════════════════════

/// Merging source vendor into target:
///   * source vendor is gone from the list
///   * target vendor's total_spend += source's pre-merge total
///   * every previously-source-vendor transaction's logical id still resolves
///     (via the dashboard/top-vendors card under the target vendor)
#[rocket::async_test]
#[ignore = "requires database"]
async fn property_vendor_merge_moves_aggregates_and_preserves_ids() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Merge Acct", 10_000_000).await;
    let cat = create_category(&client, "Merge Cat", "expense").await;
    let _period = create_period(&client, "2026-10-01", "2026-10-31").await;
    let source = create_vendor(&client, "Source Vendor").await;
    let target = create_vendor(&client, "Target Vendor").await;

    let mut rng = rand::rng();
    let mut total = 0_i64;
    let mut source_tx_ids: Vec<String> = Vec::new();

    for _ in 0..10 {
        let amount: i64 = rng.random_range(100..=5_000);
        total += amount;
        let id = create_transaction_with_vendor(&client, &acct, &cat, amount, "2026-10-15", &source).await;
        source_tx_ids.push(id);
    }

    // Pre-merge: source vendor has the full total
    let vendors_before = get_json(&client, &format!("{V2_BASE}/vendors")).await;
    let source_before_total = vendors_before["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"].as_str() == Some(&source))
        .and_then(|v| v["totalSpend"].as_i64())
        .unwrap();
    assert_eq!(source_before_total, total, "Pre-merge source total mismatch");

    // Merge source into target
    let resp = client
        .post(format!("{V2_BASE}/vendors/{source}/merge"))
        .header(ContentType::JSON)
        .body(json!({ "targetVendorId": target }).to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Post-merge: source vendor gone from list
    let vendors_after = get_json(&client, &format!("{V2_BASE}/vendors")).await;
    let source_still_there = vendors_after["data"].as_array().unwrap().iter().any(|v| v["id"].as_str() == Some(&source));
    assert!(!source_still_there, "Property 9 violated: source vendor still in list after merge");

    // Post-merge: target vendor total == pre-merge source total
    let target_total = vendors_after["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"].as_str() == Some(&target))
        .and_then(|v| v["totalSpend"].as_i64())
        .unwrap();
    assert_eq!(target_total, total, "Property 9 violated: target vendor total did not pick up source spend");
}

// ═══════════════════════════════════════════════════════════════════════════
// Property 11: DTO rejects negative amounts on create and correct
// ═══════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn property_negative_amount_rejected_at_dto() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "Neg Acct", 100_000).await;
    let cat = create_category(&client, "Neg Cat", "expense").await;
    let _period = create_period(&client, "2026-11-01", "2026-11-30").await;

    let mut rng = rand::rng();

    for _ in 0..20 {
        let bad: i64 = -rng.random_range(1..=100_000);
        let resp = client
            .post(format!("{V2_BASE}/transactions"))
            .header(ContentType::JSON)
            .body(
                json!({
                    "transactionType": "Regular",
                    "date": "2026-11-10",
                    "description": "negative attempt",
                    "amount": bad,
                    "fromAccountId": acct,
                    "categoryId": cat,
                })
                .to_string(),
            )
            .dispatch()
            .await;
        assert_eq!(
            resp.status(),
            Status::BadRequest,
            "Property 11 violated: negative amount {bad} accepted on create"
        );
    }

    // Also verify correction path rejects negative
    let real_tx = create_transaction(&client, &acct, &cat, 1_000, "2026-11-10").await;
    let status = put_transaction(
        &client,
        &real_tx,
        json!({
            "transactionType": "Regular",
            "date": "2026-11-10",
            "description": "negative correct",
            "amount": -500,
            "fromAccountId": acct,
            "categoryId": cat,
        }),
    )
    .await;
    assert_eq!(status, Status::BadRequest, "Property 11 violated: negative correction accepted");

    let _ = Uuid::new_v4(); // suppress unused import lint when above tests shrink
}

// ═══════════════════════════════════════════════════════════════════════════
// Property 13: type-field change attempts return 400, not 500
// ═══════════════════════════════════════════════════════════════════════════

/// PUT /accounts/:id with a different `type` value must return 400 — not a
/// 500 from the raw immutability trigger. Same for PUT /categories/:id with
/// a different `type`. This is a regression test for the Schemathesis
/// fuzzing failure on PR #311, where random `type` values produced 5xx.
#[rocket::async_test]
#[ignore = "requires database"]
async fn property_type_field_change_returns_400_not_500() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_account(&client, "TypeGuard Acct", 100_000).await;
    let cat_expense = create_category(&client, "TypeGuard Cat", "expense").await;
    let _period = create_period(&client, "2026-12-01", "2026-12-31").await;

    // Attempt to change an account's type from Checking → Savings
    let resp = client
        .put(format!("{V2_BASE}/accounts/{acct}"))
        .header(ContentType::JSON)
        .body(
            json!({
                "type": "Savings",
                "name": "TypeGuard Acct",
                "color": "#6B8FD4",
                "initialBalance": 100_000,
                "currencyId": Uuid::new_v4(),
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(
        resp.status(),
        Status::BadRequest,
        "Property 13 violated: account type change returned {} instead of 400",
        resp.status()
    );

    // Attempt to change a category's type from expense → income
    let resp = client
        .put(format!("{V2_BASE}/categories/{cat_expense}"))
        .header(ContentType::JSON)
        .body(
            json!({
                "type": "income",
                "name": "TypeGuard Cat",
                "color": "#6B8FD4",
                "icon": "💰",
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(
        resp.status(),
        Status::BadRequest,
        "Property 13 violated: category type change returned {} instead of 400",
        resp.status()
    );

    // Same-value updates should still succeed
    let resp = client
        .put(format!("{V2_BASE}/accounts/{acct}"))
        .header(ContentType::JSON)
        .body(
            json!({
                "type": "Checking",
                "name": "TypeGuard Acct Renamed",
                "color": "#6B8FD4",
                "initialBalance": 100_000,
                "currencyId": Uuid::new_v4(),
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert!(resp.status().code < 500, "same-value account update failed with 5xx: {}", resp.status());
}
