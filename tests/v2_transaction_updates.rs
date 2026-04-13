//! Priority 2: Update-transaction multi-hop characterization tests.
//!
//! When a transaction is mutated, every derived value that references it
//! (account balances, period spent, cash flow, top-vendors, fixed/variable
//! categorization) must move in lockstep. These tests lock in the current v2
//! behavior ahead of the ledger refactor.

mod common;

use common::auth::{create_user_and_login, get_eur_currency_id};
use common::entities::{create_account, create_category, create_period, create_transaction, create_transaction_with_vendor, create_vendor};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::{Value, json};

async fn get_json(client: &Client, url: &str) -> Value {
    let resp = client.get(url.to_string()).dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "GET {url} failed with {}", resp.status());
    serde_json::from_str(&resp.into_string().await.unwrap()).unwrap()
}

/// Fetches the account's currentBalance via the summary endpoint. Requires a
/// period to scope the summary query; any valid period ID works.
async fn get_account_balance(client: &Client, account_id: &str, period_id: &str) -> i64 {
    let body = get_json(client, &format!("{V2_BASE}/accounts/summary?periodId={period_id}")).await;
    let data = body["data"].as_array().expect("summary data array");
    let acct = data.iter().find(|a| a["id"].as_str() == Some(account_id)).expect("account in summary");
    acct["currentBalance"].as_i64().expect("currentBalance")
}

async fn put_transaction(client: &Client, tx_id: &str, payload: Value) {
    let resp = client
        .put(format!("{V2_BASE}/transactions/{tx_id}"))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok, "PUT transaction failed with {}", resp.status());
}

async fn create_fixed_expense_category(client: &Client, name: &str) -> String {
    let payload = json!({
        "name": name,
        "type": "expense",
        "behavior": "fixed",
        "icon": "🏠",
        "description": null,
        "parentId": null
    });
    let resp = client
        .post(format!("{V2_BASE}/categories"))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    body["id"].as_str().unwrap().to_string()
}

async fn create_checking(client: &Client, name: &str, initial: i64) -> String {
    create_account(client, name, initial).await
}

async fn create_allowance(client: &Client, name: &str, initial: i64) -> String {
    let eur = get_eur_currency_id(client).await;
    let payload = json!({
        "type": "Allowance",
        "name": name,
        "color": "#112233",
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
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    body["id"].as_str().unwrap().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Update amount
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn update_amount_moves_balance_and_spent() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_checking(&client, "UA Checking", 100_000).await;
    let cat = create_category(&client, "UA Expense", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let tx_id = create_transaction(&client, &acct, &cat, 5_000, "2026-03-10").await;
    assert_eq!(get_account_balance(&client, &acct, &period_id).await, 95_000);
    let cp = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(cp["spent"], 5_000);

    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-03-10",
            "description": "Test transaction",
            "amount": 12_000,
            "fromAccountId": acct,
            "categoryId": cat,
            "vendorId": null
        }),
    )
    .await;

    assert_eq!(
        get_account_balance(&client, &acct, &period_id).await,
        88_000,
        "account should reflect new amount"
    );
    let cp = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(cp["spent"], 12_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// Update category variable → fixed
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn update_category_variable_to_fixed_shifts_spent_classification() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_checking(&client, "VF Checking", 100_000).await;
    let variable_cat = create_category(&client, "Groceries VF", "expense").await;
    let fixed_cat = create_fixed_expense_category(&client, "Rent VF").await;
    // Give the fixed category a target so it appears in the endpoint
    common::entities::create_target(&client, &fixed_cat, 50_000).await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let tx_id = create_transaction(&client, &acct, &variable_cat, 10_000, "2026-03-05").await;

    // Baseline: rent fixed category has 0 spent
    let fixed_before = get_json(&client, &format!("{}/dashboard/fixed-categories?periodId={}", V2_BASE, period_id)).await;
    let rent_before = fixed_before.as_array().unwrap().iter().find(|c| c["categoryId"] == fixed_cat.as_str()).unwrap();
    assert_eq!(rent_before["spent"], 0);

    // Reassign tx to the fixed category
    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-03-05",
            "description": "Test transaction",
            "amount": 10_000,
            "fromAccountId": acct,
            "categoryId": fixed_cat,
            "vendorId": null
        }),
    )
    .await;

    let fixed_after = get_json(&client, &format!("{}/dashboard/fixed-categories?periodId={}", V2_BASE, period_id)).await;
    let rent_after = fixed_after.as_array().unwrap().iter().find(|c| c["categoryId"] == fixed_cat.as_str()).unwrap();
    assert_eq!(rent_after["spent"], 10_000, "fixed category should now reflect the tx");

    // total spent on period unchanged (same amount, same account)
    let cp = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(cp["spent"], 10_000);
    assert_eq!(get_account_balance(&client, &acct, &period_id).await, 90_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// Update fromAccount
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn update_from_account_reverts_old_and_applies_new() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct_a = create_checking(&client, "FA Acct A", 100_000).await;
    let acct_b = create_checking(&client, "FA Acct B", 50_000).await;
    let cat = create_category(&client, "FA Cat", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let tx_id = create_transaction(&client, &acct_a, &cat, 7_500, "2026-03-08").await;
    assert_eq!(get_account_balance(&client, &acct_a, &period_id).await, 92_500);
    assert_eq!(get_account_balance(&client, &acct_b, &period_id).await, 50_000);

    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-03-08",
            "description": "Test transaction",
            "amount": 7_500,
            "fromAccountId": acct_b,
            "categoryId": cat,
            "vendorId": null
        }),
    )
    .await;

    assert_eq!(get_account_balance(&client, &acct_a, &period_id).await, 100_000, "source acct A should revert");
    assert_eq!(
        get_account_balance(&client, &acct_b, &period_id).await,
        42_500,
        "dest acct B should now carry the expense"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Update toAccount on a transfer
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn update_transfer_to_account_swaps_balances_correctly() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let src = create_checking(&client, "TX Src", 100_000).await;
    let dest1 = create_allowance(&client, "TX Dest1", 0).await;
    let dest2 = create_allowance(&client, "TX Dest2", 0).await;
    let transfer_cat = create_category(&client, "TX Transfer", "transfer").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    // Create transfer src → dest1
    let resp = client
        .post(format!("{V2_BASE}/transactions"))
        .header(ContentType::JSON)
        .body(
            json!({
                "transactionType": "Transfer",
                "date": "2026-03-12",
                "description": "first transfer",
                "amount": 20_000,
                "fromAccountId": src,
                "categoryId": transfer_cat,
                "vendorId": null,
                "toAccountId": dest1,
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);
    let tx_id = serde_json::from_str::<Value>(&resp.into_string().await.unwrap()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    assert_eq!(get_account_balance(&client, &src, &period_id).await, 80_000);
    assert_eq!(get_account_balance(&client, &dest1, &period_id).await, 20_000);
    assert_eq!(get_account_balance(&client, &dest2, &period_id).await, 0);

    // Repoint to dest2
    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Transfer",
            "date": "2026-03-12",
            "description": "first transfer",
            "amount": 20_000,
            "fromAccountId": src,
            "categoryId": transfer_cat,
            "vendorId": null,
            "toAccountId": dest2,
        }),
    )
    .await;

    assert_eq!(get_account_balance(&client, &src, &period_id).await, 80_000, "source unchanged");
    assert_eq!(get_account_balance(&client, &dest1, &period_id).await, 0, "old dest reverts");
    assert_eq!(get_account_balance(&client, &dest2, &period_id).await, 20_000, "new dest receives");
}

// ─────────────────────────────────────────────────────────────────────────────
// Update occurred_at OUT of current period
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn update_date_out_of_period_removes_from_period_aggregations() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_checking(&client, "OP Checking", 100_000).await;
    let cat = create_category(&client, "OP Expense", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let tx_id = create_transaction(&client, &acct, &cat, 4_000, "2026-03-10").await;
    let cp = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(cp["spent"], 4_000);

    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-04-10", // outside period
            "description": "Test transaction",
            "amount": 4_000,
            "fromAccountId": acct,
            "categoryId": cat,
            "vendorId": null
        }),
    )
    .await;

    let cp = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(cp["spent"], 0, "period spent should drop to 0 when tx moves out");

    // Transaction should no longer appear when listing by period
    let list = get_json(&client, &format!("{}/transactions?periodId={}", V2_BASE, period_id)).await;
    let data = list["data"].as_array().unwrap();
    assert!(data.iter().all(|t| t["id"].as_str().unwrap() != tx_id), "tx must not appear in period list");

    // Account balance is unchanged by the date move
    assert_eq!(get_account_balance(&client, &acct, &period_id).await, 96_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// Update occurred_at INTO current period
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn update_date_into_period_includes_in_aggregations() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_checking(&client, "IP Checking", 100_000).await;
    let cat = create_category(&client, "IP Expense", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    // Starts outside the period
    let tx_id = create_transaction(&client, &acct, &cat, 3_000, "2026-02-10").await;
    let cp = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(cp["spent"], 0);

    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-03-15",
            "description": "Test transaction",
            "amount": 3_000,
            "fromAccountId": acct,
            "categoryId": cat,
            "vendorId": null
        }),
    )
    .await;

    let cp = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(cp["spent"], 3_000);
    assert_eq!(get_account_balance(&client, &acct, &period_id).await, 97_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// Update vendor
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn update_vendor_shifts_top_vendors_counts() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct = create_checking(&client, "UV Checking", 100_000).await;
    let cat = create_category(&client, "UV Expense", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let vendor_a = create_vendor(&client, "UV Alpha").await;
    let vendor_b = create_vendor(&client, "UV Beta").await;

    let tx_id = create_transaction_with_vendor(&client, &acct, &cat, 9_000, "2026-03-10", &vendor_a).await;

    let tv_before = get_json(&client, &format!("{}/dashboard/top-vendors?periodId={}", V2_BASE, period_id)).await;
    let a_before = tv_before.as_array().unwrap().iter().find(|v| v["vendorId"] == vendor_a.as_str()).unwrap();
    assert_eq!(a_before["totalSpent"], 9_000);
    assert_eq!(a_before["transactionCount"], 1);
    assert!(tv_before.as_array().unwrap().iter().all(|v| v["vendorId"] != vendor_b.as_str()));

    // Reassign to vendor B
    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-03-10",
            "description": "Test transaction",
            "amount": 9_000,
            "fromAccountId": acct,
            "categoryId": cat,
            "vendorId": vendor_b,
        }),
    )
    .await;

    let tv_after = get_json(&client, &format!("{}/dashboard/top-vendors?periodId={}", V2_BASE, period_id)).await;
    assert!(
        tv_after.as_array().unwrap().iter().all(|v| v["vendorId"] != vendor_a.as_str()),
        "vendor A must no longer appear"
    );
    let b_after = tv_after.as_array().unwrap().iter().find(|v| v["vendorId"] == vendor_b.as_str()).unwrap();
    assert_eq!(b_after["totalSpent"], 9_000);
    assert_eq!(b_after["transactionCount"], 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Round-trip: update + reverse-update should restore original balances
// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn round_trip_update_restores_original_balances() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let acct_a = create_checking(&client, "RT Acct A", 100_000).await;
    let acct_b = create_checking(&client, "RT Acct B", 50_000).await;
    let cat = create_category(&client, "RT Cat", "expense").await;
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let tx_id = create_transaction(&client, &acct_a, &cat, 6_000, "2026-03-10").await;

    let a0 = get_account_balance(&client, &acct_a, &period_id).await;
    let b0 = get_account_balance(&client, &acct_b, &period_id).await;

    // Move to B, change amount
    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-03-11",
            "description": "round trip",
            "amount": 11_000,
            "fromAccountId": acct_b,
            "categoryId": cat,
            "vendorId": null
        }),
    )
    .await;

    // Reverse
    put_transaction(
        &client,
        &tx_id,
        json!({
            "transactionType": "Regular",
            "date": "2026-03-10",
            "description": "Test transaction",
            "amount": 6_000,
            "fromAccountId": acct_a,
            "categoryId": cat,
            "vendorId": null
        }),
    )
    .await;

    assert_eq!(get_account_balance(&client, &acct_a, &period_id).await, a0, "A balance round-trips");
    assert_eq!(get_account_balance(&client, &acct_b, &period_id).await, b0, "B balance round-trips");
}
