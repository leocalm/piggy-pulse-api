mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_period, create_target, create_transaction};
use common::{TEST_PASSWORD, V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// GET a JSON endpoint and return parsed value. Asserts 200.
async fn get_json(client: &Client, url: &str) -> Value {
    let resp = client.get(url.to_string()).dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "GET {} failed with {}", url, resp.status());
    serde_json::from_str(&resp.into_string().await.unwrap()).unwrap()
}

/// Create a transfer transaction. Returns the transaction ID.
async fn create_transfer(client: &Client, from_account_id: &str, to_account_id: &str, category_id: &str, amount: i64, date: &str) -> String {
    let payload = serde_json::json!({
        "transactionType": "Transfer",
        "date": date,
        "description": "Transfer",
        "amount": amount,
        "fromAccountId": from_account_id,
        "categoryId": category_id,
        "vendorId": null,
        "toAccountId": to_account_id
    });
    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_transfer failed");
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    body["id"].as_str().expect("transaction id").to_string()
}

/// Find an account in the summaries list by ID and return its currentBalance.
async fn get_account_current_balance(client: &Client, account_id: &str, period_id: &str) -> i64 {
    let body = get_json(client, &format!("{}/accounts/summary?periodId={}", V2_BASE, period_id)).await;
    let data = body["data"].as_array().expect("data array");
    let account = data
        .iter()
        .find(|a| a["id"].as_str() == Some(account_id))
        .unwrap_or_else(|| panic!("account {} not found in summaries", account_id));
    account["currentBalance"].as_i64().expect("currentBalance")
}

/// Get transaction stats for a period (totalInflows, totalOutflows, netAmount, transactionCount).
async fn get_tx_stats(client: &Client, period_id: &str) -> Value {
    get_json(client, &format!("{}/transactions/stats?periodId={}", V2_BASE, period_id)).await
}

/// Get cash-flow for a period (inflows, outflows, net).
async fn get_cash_flow(client: &Client, period_id: &str) -> Value {
    get_json(client, &format!("{}/dashboard/cash-flow?periodId={}", V2_BASE, period_id)).await
}

// ═══════════════════════════════════════════════════════════════════════════════
// Group 1: Transaction → Dashboard Cascade
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_outgoing_transaction_increases_dashboard_spend() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Checking", 200_000).await;
    let expense_cat = create_category(&client, "DI Groceries", "expense").await;

    // Baseline: no spending yet
    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalOutflows"], 0);

    let dashboard = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(dashboard["spent"], 0);

    // Action: create an outgoing transaction of EUR 85.50 (8550 cents)
    create_transaction(&client, &account_id, &expense_cat, 8_550, "2026-04-06").await;

    // Assert: dashboard and stats reflect the spend
    let dashboard = get_json(&client, &format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(dashboard["spent"], 8_550);

    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalOutflows"], 8_550);
    assert_eq!(stats["totalInflows"], 0);
    assert_eq!(stats["netAmount"], -8_550);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_incoming_transaction_increases_dashboard_income() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Income Acct", 200_000).await;
    let income_cat = create_category(&client, "DI Salary", "income").await;

    // Action: create an income transaction of EUR 3000 (300_000 cents)
    create_transaction(&client, &account_id, &income_cat, 300_000, "2026-04-05").await;

    // Assert
    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalInflows"], 300_000);
    assert_eq!(stats["totalOutflows"], 0);
    assert_eq!(stats["netAmount"], 300_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_transaction_recalculates_dashboard() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Del Acct", 200_000).await;
    let expense_cat = create_category(&client, "DI Del Cat", "expense").await;

    // Create 3 transactions: EUR 100, EUR 50, EUR 25
    let _tx1 = create_transaction(&client, &account_id, &expense_cat, 10_000, "2026-04-05").await;
    let tx2 = create_transaction(&client, &account_id, &expense_cat, 5_000, "2026-04-06").await;
    let _tx3 = create_transaction(&client, &account_id, &expense_cat, 2_500, "2026-04-07").await;

    // Baseline: total = 17500
    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalOutflows"], 17_500);

    // Action: delete the EUR 50 transaction
    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, tx2)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Assert: total = 12500 (100 + 25)
    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalOutflows"], 12_500);
    assert_eq!(stats["transactionCount"], 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_edit_transaction_amount_recalculates_dashboard() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Edit Acct", 200_000).await;
    let expense_cat = create_category(&client, "DI Edit Cat", "expense").await;

    // Create transaction: EUR 85.50 (8550)
    let tx_id = create_transaction(&client, &account_id, &expense_cat, 8_550, "2026-04-10").await;

    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalOutflows"], 8_550);

    // Action: update amount to EUR 120 (12000)
    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-04-10",
        "description": "Updated groceries",
        "amount": 12_000,
        "fromAccountId": account_id,
        "categoryId": expense_cat,
        "vendorId": null
    });
    let resp = client
        .put(format!("{}/transactions/{}", V2_BASE, tx_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Assert
    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalOutflows"], 12_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_edit_transaction_category_moves_spending() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI CatMove Acct", 200_000).await;
    let groceries_cat = create_category(&client, "DI Groceries CM", "expense").await;
    let transport_cat = create_category(&client, "DI Transport CM", "expense").await;

    // Create transaction under Groceries: EUR 50 (5000)
    let tx_id = create_transaction(&client, &account_id, &groceries_cat, 5_000, "2026-04-10").await;

    // Verify Groceries has spend via category overview
    let overview = get_json(&client, &format!("{}/categories/overview?periodId={}", V2_BASE, period_id)).await;
    let cats = overview["categories"].as_array().unwrap();
    let groceries = cats.iter().find(|c| c["name"] == "DI Groceries CM").unwrap();
    assert_eq!(groceries["actual"], 5_000);

    // Action: move transaction to Transport category
    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-04-10",
        "description": "Test transaction",
        "amount": 5_000,
        "fromAccountId": account_id,
        "categoryId": transport_cat,
        "vendorId": null
    });
    let resp = client
        .put(format!("{}/transactions/{}", V2_BASE, tx_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Assert: Groceries = 0, Transport = 5000
    let overview = get_json(&client, &format!("{}/categories/overview?periodId={}", V2_BASE, period_id)).await;
    let cats = overview["categories"].as_array().unwrap();
    let groceries = cats.iter().find(|c| c["name"] == "DI Groceries CM").unwrap();
    let transport = cats.iter().find(|c| c["name"] == "DI Transport CM").unwrap();
    assert_eq!(groceries["actual"], 0);
    assert_eq!(transport["actual"], 5_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_multiple_transactions_aggregate_correctly() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Multi Acct", 1_000_000).await;

    let salary_cat = create_category(&client, "DI Salary Multi", "income").await;
    let rent_cat = create_category(&client, "DI Rent Multi", "expense").await;
    let groceries_cat = create_category(&client, "DI Groc Multi", "expense").await;
    let transport_cat = create_category(&client, "DI Trans Multi", "expense").await;

    // Salary: +EUR 3000
    create_transaction(&client, &account_id, &salary_cat, 300_000, "2026-04-01").await;
    // Rent: -EUR 1200
    create_transaction(&client, &account_id, &rent_cat, 120_000, "2026-04-02").await;
    // Groceries: -EUR 85.50
    create_transaction(&client, &account_id, &groceries_cat, 8_550, "2026-04-03").await;
    // Transport: -EUR 35
    create_transaction(&client, &account_id, &transport_cat, 3_500, "2026-04-04").await;

    // Assert via transaction stats
    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalInflows"], 300_000);
    assert_eq!(stats["totalOutflows"], 132_050); // 120000 + 8550 + 3500
    assert_eq!(stats["netAmount"], 167_950); // 300000 - 132050

    // Assert via cash-flow
    let cf = get_cash_flow(&client, &period_id).await;
    assert_eq!(cf["inflows"], 300_000);
    assert_eq!(cf["outflows"], 132_050);
    assert_eq!(cf["net"], 167_950);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Group 2: Transaction → Account Balance Cascade
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_outgoing_transaction_decreases_account_balance() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI BalDec Acct", 200_000).await; // EUR 2000
    let expense_cat = create_category(&client, "DI BalDec Cat", "expense").await;

    // Baseline
    let balance_before = get_account_current_balance(&client, &account_id, &period_id).await;
    assert_eq!(balance_before, 200_000);

    // Action: spend EUR 85.50
    create_transaction(&client, &account_id, &expense_cat, 8_550, "2026-04-06").await;

    // Assert: balance = 200000 - 8550 = 191450
    let balance_after = get_account_current_balance(&client, &account_id, &period_id).await;
    assert_eq!(balance_after, 191_450);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_incoming_transaction_increases_account_balance() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI BalInc Acct", 200_000).await; // EUR 2000
    let income_cat = create_category(&client, "DI BalInc Salary", "income").await;

    // Action: receive EUR 3000
    create_transaction(&client, &account_id, &income_cat, 300_000, "2026-04-05").await;

    // Assert: balance = 200000 + 300000 = 500000
    let balance = get_account_current_balance(&client, &account_id, &period_id).await;
    assert_eq!(balance, 500_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_transfer_moves_balance_between_accounts() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let checking_id = create_account(&client, "DI Xfer Checking", 200_000).await; // EUR 2000
    let savings_id = create_account(&client, "DI Xfer Savings", 500_000).await; // EUR 5000
    let transfer_cat = create_category(&client, "DI Xfer Cat", "transfer").await;

    // Pre-transfer net position
    let net_before = get_json(&client, &format!("{}/dashboard/net-position?periodId={}", V2_BASE, period_id)).await;
    let total_before = net_before["total"].as_i64().unwrap();

    // Action: transfer EUR 500 from Checking to Savings
    create_transfer(&client, &checking_id, &savings_id, &transfer_cat, 50_000, "2026-04-06").await;

    // Assert: Checking decreased by EUR 500
    let checking_balance = get_account_current_balance(&client, &checking_id, &period_id).await;
    assert_eq!(checking_balance, 150_000); // 200000 - 50000

    // Assert: Savings increased by EUR 500
    let savings_balance = get_account_current_balance(&client, &savings_id, &period_id).await;
    assert_eq!(savings_balance, 550_000); // 500000 + 50000

    // Assert: net position unchanged (transfers are neutral)
    let net_after = get_json(&client, &format!("{}/dashboard/net-position?periodId={}", V2_BASE, period_id)).await;
    assert_eq!(net_after["total"].as_i64().unwrap(), total_before);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_transaction_restores_account_balance() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Restore Acct", 200_000).await; // EUR 2000
    let expense_cat = create_category(&client, "DI Restore Cat", "expense").await;

    // Create expense of EUR 100
    let tx_id = create_transaction(&client, &account_id, &expense_cat, 10_000, "2026-04-06").await;

    // Balance after tx = 190000
    let balance_with_tx = get_account_current_balance(&client, &account_id, &period_id).await;
    assert_eq!(balance_with_tx, 190_000);

    // Action: delete the transaction
    let resp = client.delete(format!("{}/transactions/{}", V2_BASE, tx_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Assert: balance restored to 200000
    let balance_restored = get_account_current_balance(&client, &account_id, &period_id).await;
    assert_eq!(balance_restored, 200_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_edit_transaction_account_moves_balance() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_a = create_account(&client, "DI MoveA Acct", 200_000).await; // EUR 2000
    let account_b = create_account(&client, "DI MoveB Acct", 300_000).await; // EUR 3000
    let expense_cat = create_category(&client, "DI Move Cat", "expense").await;

    // Create EUR 100 expense from Account A
    let tx_id = create_transaction(&client, &account_a, &expense_cat, 10_000, "2026-04-06").await;

    // Baseline: A = 190000, B = 300000
    assert_eq!(get_account_current_balance(&client, &account_a, &period_id).await, 190_000);
    assert_eq!(get_account_current_balance(&client, &account_b, &period_id).await, 300_000);

    // Action: move transaction from A to B
    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-04-06",
        "description": "Test transaction",
        "amount": 10_000,
        "fromAccountId": account_b,
        "categoryId": expense_cat,
        "vendorId": null
    });
    let resp = client
        .put(format!("{}/transactions/{}", V2_BASE, tx_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Assert: A restored to 200000, B decreased to 290000
    assert_eq!(get_account_current_balance(&client, &account_a, &period_id).await, 200_000);
    assert_eq!(get_account_current_balance(&client, &account_b, &period_id).await, 290_000);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Group 3: Budget Category Targets
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_budget_allocation_equals_sum_of_targets() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let _account_id = create_account(&client, "DI Target Acct", 200_000).await;

    let groceries = create_category(&client, "DI Groc Tgt", "expense").await;
    let rent = create_category(&client, "DI Rent Tgt", "expense").await;
    let transport = create_category(&client, "DI Trans Tgt", "expense").await;

    // Set targets: Groceries EUR 400, Rent EUR 1200, Transport EUR 100
    create_target(&client, &groceries, 40_000).await;
    create_target(&client, &rent, 120_000).await;
    create_target(&client, &transport, 10_000).await;

    // Assert: targets endpoint reflects all three
    let targets = get_json(&client, &format!("{}/targets?periodId={}", V2_BASE, period_id)).await;
    let target_list = targets["targets"].as_array().unwrap();

    // Count active targets with values we set
    let active: Vec<&Value> = target_list.iter().filter(|t| t["status"] == "active").collect();
    assert_eq!(active.len(), 3, "expected 3 active targets");

    // Sum of all currentTarget values = 170000 (EUR 1700)
    let total: i64 = active.iter().map(|t| t["currentTarget"].as_i64().unwrap()).sum();
    assert_eq!(total, 170_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_spending_reflected_in_category_targets() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI SpendTgt Acct", 200_000).await;
    let groceries = create_category(&client, "DI SpendTgt Groc", "expense").await;

    // Set target: Groceries EUR 400 (40000)
    create_target(&client, &groceries, 40_000).await;

    // Create 2 transactions: EUR 50 + EUR 30 = EUR 80 (8000)
    create_transaction(&client, &account_id, &groceries, 5_000, "2026-04-05").await;
    create_transaction(&client, &account_id, &groceries, 3_000, "2026-04-06").await;

    // Assert: target shows spend
    let targets = get_json(&client, &format!("{}/targets?periodId={}", V2_BASE, period_id)).await;
    let target_list = targets["targets"].as_array().unwrap();
    let groc_target = target_list.iter().find(|t| t["name"] == "DI SpendTgt Groc").expect("groceries target");
    assert_eq!(groc_target["currentTarget"], 40_000);
    assert_eq!(groc_target["spentInPeriod"], 8_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_target_removes_from_active() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let _account_id = create_account(&client, "DI ExclTgt Acct", 200_000).await;

    let cat1 = create_category(&client, "DI ExclTgt A", "expense").await;
    let cat2 = create_category(&client, "DI ExclTgt B", "expense").await;

    let target1_id = create_target(&client, &cat1, 20_000).await;
    create_target(&client, &cat2, 30_000).await;

    // Baseline: 2 active targets
    let targets = get_json(&client, &format!("{}/targets?periodId={}", V2_BASE, period_id)).await;
    let active_count = targets["targets"].as_array().unwrap().iter().filter(|t| t["status"] == "active").count();
    assert_eq!(active_count, 2);

    // Action: exclude target 1
    let resp = client.post(format!("{}/targets/{}/exclude", V2_BASE, target1_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    // Assert: 1 active target remains, the excluded one has status "excluded"
    let targets = get_json(&client, &format!("{}/targets?periodId={}", V2_BASE, period_id)).await;
    let target_list = targets["targets"].as_array().unwrap();
    let active_count = target_list.iter().filter(|t| t["status"] == "active").count();
    assert_eq!(active_count, 1);

    let excluded = target_list.iter().find(|t| t["id"] == target1_id).unwrap();
    assert_eq!(excluded["status"], "excluded");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Group 4: Period Boundary Behavior
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_transaction_outside_period_not_counted_in_summary() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Period: April 1-30
    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI OutPeriod Acct", 200_000).await;
    let expense_cat = create_category(&client, "DI OutPeriod Cat", "expense").await;

    // Create transaction in March (outside the April period)
    create_transaction(&client, &account_id, &expense_cat, 5_000, "2026-03-15").await;

    // Assert: April stats show zero
    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalOutflows"], 0);
    assert_eq!(stats["transactionCount"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_transaction_on_period_boundary_included() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Period: April 1-30
    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Boundary Acct", 200_000).await;
    let expense_cat = create_category(&client, "DI Boundary Cat", "expense").await;

    // Create transactions on first and last day of the period
    create_transaction(&client, &account_id, &expense_cat, 3_000, "2026-04-01").await;
    create_transaction(&client, &account_id, &expense_cat, 7_000, "2026-04-30").await;

    // Assert: both included in the period
    let stats = get_tx_stats(&client, &period_id).await;
    assert_eq!(stats["totalOutflows"], 10_000);
    assert_eq!(stats["transactionCount"], 2);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_switching_period_returns_different_data() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let march_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let april_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Switch Acct", 200_000).await;
    let expense_cat = create_category(&client, "DI Switch Cat", "expense").await;

    // March: EUR 100
    create_transaction(&client, &account_id, &expense_cat, 10_000, "2026-03-15").await;
    // April: EUR 200
    create_transaction(&client, &account_id, &expense_cat, 20_000, "2026-04-15").await;

    // Assert: each period has its own totals
    let march_stats = get_tx_stats(&client, &march_id).await;
    assert_eq!(march_stats["totalOutflows"], 10_000);

    let april_stats = get_tx_stats(&client, &april_id).await;
    assert_eq!(april_stats["totalOutflows"], 20_000);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Group 5: Validation & Error Cases
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_transaction_negative_amount_rejected() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let account_id = create_account(&client, "DI NegAmt Acct", 100_000).await;
    let expense_cat = create_category(&client, "DI NegAmt Cat", "expense").await;

    let payload = serde_json::json!({
        "transactionType": "Regular",
        "date": "2026-04-06",
        "description": "Negative amount",
        "amount": -100,
        "fromAccountId": account_id,
        "categoryId": expense_cat,
        "vendorId": null
    });

    let resp = client
        .post(format!("{}/transactions", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400 or 422 for negative amount, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_end_before_start_rejected() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-04-30",
        "name": "Backwards Period",
        "manualEndDate": "2026-04-01"
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400 or 422 for end-before-start period, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_category_with_transactions_blocked() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let _period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI DelCat Acct", 100_000).await;
    let category_id = create_category(&client, "DI DelCat Cat", "expense").await;

    // Create a transaction using this category
    create_transaction(&client, &account_id, &category_id, 5_000, "2026-04-10").await;

    // Action: attempt to delete the category
    let resp = client.delete(format!("{}/categories/{}", V2_BASE, category_id)).dispatch().await;

    // Should be blocked because the category has transactions
    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::Conflict || resp.status() == Status::Forbidden,
        "expected 400, 403, or 409 for deleting category with transactions, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_register_duplicate_email_rejected() {
    let client = test_client().await;
    let (_user_id, email) = create_user_and_login(&client).await;

    // Try to register again with the same email (from a fresh client to avoid session conflicts)
    let client2 = test_client().await;
    let payload = serde_json::json!({
        "name": "Duplicate User",
        "email": email,
        "password": TEST_PASSWORD
    });

    let resp = client2
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::Conflict || resp.status() == Status::BadRequest,
        "expected 409 or 400 for duplicate email, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_register_weak_password_rejected() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "name": "Weak Password User",
        "email": format!("weak.{}@example.com", uuid::Uuid::new_v4()),
        "password": "123456"
    });

    let resp = client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400 or 422 for weak password, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_login_wrong_password_returns_401() {
    let client = test_client().await;
    let (_user_id, email) = create_user_and_login(&client).await;

    // Login with wrong password from a fresh client
    let client2 = test_client().await;
    let payload = serde_json::json!({
        "email": email,
        "password": "WrongPassword!2026" // pragma: allowlist secret
    });

    let resp = client2
        .post(format!("{}/auth/login", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    // 401 = wrong credentials, 423 = progressive backoff locked
    assert!(
        resp.status() == Status::Unauthorized || resp.status() == Status::Locked,
        "expected 401 or 423, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unauthenticated_request_returns_401() {
    let client = test_client().await;

    // No login — try to access a protected endpoint
    let resp = client.get(format!("{}/accounts", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_accessing_other_users_data_returns_404() {
    // User A creates an account
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let account_id = create_account(&client_a, "DI UserA Private", 100_000).await;

    // User B tries to access User A's account
    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/accounts/{}", V2_BASE, account_id)).dispatch().await;

    // Should be 404 (not 403) — don't leak existence
    assert_eq!(resp.status(), Status::NotFound);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Group 6: Data Export Integrity
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_csv_export_matches_transaction_data() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let _period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI CSV Acct", 100_000).await;
    let category_id = create_category(&client, "DI CSV Cat", "expense").await;

    // Create 3 transactions with known amounts
    create_transaction(&client, &account_id, &category_id, 1_000, "2026-04-05").await;
    create_transaction(&client, &account_id, &category_id, 2_500, "2026-04-10").await;
    create_transaction(&client, &account_id, &category_id, 7_777, "2026-04-15").await;

    // Export CSV
    let resp = client.get(format!("{}/settings/export/transactions", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let content_type = resp.content_type().expect("content type");
    assert!(content_type.to_string().contains("text/csv"), "expected text/csv, got {}", content_type);

    let body = resp.into_string().await.unwrap();

    // Parse CSV lines (skip header)
    let data_lines: Vec<&str> = body.lines().skip(1).filter(|l| !l.is_empty()).collect();
    assert_eq!(data_lines.len(), 3, "expected 3 CSV data rows");

    // Verify amounts are present in the CSV
    assert!(body.contains("1000"), "CSV should contain amount 1000");
    assert!(body.contains("2500"), "CSV should contain amount 2500");
    assert!(body.contains("7777"), "CSV should contain amount 7777");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_json_export_includes_all_entities() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let _period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "DI Export Acct", 100_000).await;
    let category_id = create_category(&client, "DI Export Cat", "expense").await;
    create_transaction(&client, &account_id, &category_id, 5_000, "2026-04-10").await;

    // Export full JSON
    let body = get_json(&client, &format!("{}/settings/export/data", V2_BASE)).await;

    // Assert: all domain arrays present and non-empty
    let accounts = body["accounts"].as_array().expect("accounts array");
    assert!(!accounts.is_empty(), "accounts should not be empty");
    assert!(accounts.iter().any(|a| a["name"] == "DI Export Acct"));

    let categories = body["categories"].as_array().expect("categories array"); // pragma: allowlist secret
    assert!(!categories.is_empty(), "categories should not be empty");
    assert!(categories.iter().any(|c| c["name"] == "DI Export Cat"));

    let transactions = body["transactions"].as_array().expect("transactions array");
    assert!(!transactions.is_empty(), "transactions should not be empty");
    assert!(transactions.iter().any(|t| t["amount"] == 5_000));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Group 7: Danger Zone
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_reset_structure_clears_financial_data() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create full data set
    let account_id = create_account(&client, "DI Reset Acct", 100_000).await;
    let category_id = create_category(&client, "DI Reset Cat", "expense").await;
    create_period(&client, "2026-04-01", "2026-04-30").await;
    create_transaction(&client, &account_id, &category_id, 5_000, "2026-04-10").await;

    // Verify data exists
    let accounts = get_json(&client, &format!("{}/accounts", V2_BASE)).await;
    assert!(!accounts["data"].as_array().unwrap().is_empty());

    // Action: reset structure (requires password confirmation)
    let payload = serde_json::json!({ "password": TEST_PASSWORD });
    let resp = client
        .post(format!("{}/settings/reset-structure", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Assert: accounts empty
    let accounts = get_json(&client, &format!("{}/accounts", V2_BASE)).await;
    assert_eq!(accounts["data"].as_array().unwrap().len(), 0);

    // Assert: transactions empty (via export — no period to query stats against)
    let export = get_json(&client, &format!("{}/settings/export/data", V2_BASE)).await;
    assert_eq!(export["transactions"].as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_reset_structure_allows_rebuilding_data() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create some data, then reset
    let cat_id = create_category(&client, "DI PreReset Cat", "expense").await;
    create_target(&client, &cat_id, 10_000).await;

    let payload = serde_json::json!({ "password": TEST_PASSWORD });
    let resp = client
        .post(format!("{}/settings/reset-structure", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Assert: can create fresh entities after reset
    let new_account = create_account(&client, "DI PostReset Acct", 50_000).await;
    let new_category = create_category(&client, "DI PostReset Cat", "expense").await;
    let new_period = create_period(&client, "2026-05-01", "2026-05-31").await;

    // Verify new entities exist and work
    let balance = get_account_current_balance(&client, &new_account, &new_period).await;
    assert_eq!(balance, 50_000);

    let _ = new_category; // confirm it was created without panic
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_account_removes_all_user_data() {
    let client = test_client().await;
    let (_user_id, email) = create_user_and_login(&client).await;

    // Create some data
    create_account(&client, "DI DelUser Acct", 100_000).await;

    // Action: delete the user account
    let payload = serde_json::json!({ "password": TEST_PASSWORD });
    let resp = client
        .delete(format!("{}/settings/account", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Assert: login with same credentials fails
    let client2 = test_client().await;
    let login_payload = serde_json::json!({
        "email": email,
        "password": TEST_PASSWORD
    });
    let resp = client2
        .post(format!("{}/auth/login", V2_BASE))
        .header(ContentType::JSON)
        .body(login_payload.to_string())
        .dispatch()
        .await;
    // 401 = credentials invalid (user deleted), 423 = progressive backoff locked
    assert!(
        resp.status() == Status::Unauthorized || resp.status() == Status::Locked,
        "expected 401 or 423 after account deletion, got {}",
        resp.status()
    );
}
