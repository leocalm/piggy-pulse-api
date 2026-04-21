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

/// Get the system Transfer category ID for the authenticated user.
async fn get_system_transfer_category_id(client: &Client) -> String {
    let body = get_json(client, &format!("{}/categories/options", V2_BASE)).await;
    // The endpoint returns { data: [...] } or just [...] depending on the route
    let options = if body.is_array() {
        body.as_array().expect("category options array")
    } else {
        body["data"].as_array().expect("category options array")
    };
    // The system Transfer category is the one named "Transfer"
    let transfer = options.iter().find(|c| c["name"] == "Transfer").expect("system Transfer category should exist");
    transfer["id"].as_str().expect("category id").to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Group 1: Transaction → Account Balance Cascade
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
    let transfer_cat = get_system_transfer_category_id(&client).await;

    // Action: transfer EUR 500 from Checking to Savings
    create_transfer(&client, &checking_id, &savings_id, &transfer_cat, 50_000, "2026-04-06").await;

    // Assert: Checking decreased by EUR 500
    let checking_balance = get_account_current_balance(&client, &checking_id, &period_id).await;
    assert_eq!(checking_balance, 150_000); // 200000 - 50000

    // Assert: Savings increased by EUR 500
    let savings_balance = get_account_current_balance(&client, &savings_id, &period_id).await;
    assert_eq!(savings_balance, 550_000); // 500000 + 50000
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
// Group 2: Budget Category Targets
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
// Group 3: Validation & Error Cases
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
    common::clear_login_rate_limits().await;
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

    assert_eq!(resp.status(), Status::Unauthorized);
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
// Group 4: Data Export Integrity
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

// ═══════════════════════════════════════════════════════════════════════════════
// Group 5: Danger Zone
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
    common::clear_login_rate_limits().await;
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
    assert_eq!(resp.status(), Status::Unauthorized);
}
