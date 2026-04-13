mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_period, create_target, create_transaction, create_transaction_with_vendor, create_vendor};
use common::{V2_BASE, test_client};
use rocket::http::Status;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/current-period
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create a period spanning today
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    // Create an expense category and a target (budget) for it
    let expense_cat = create_category(&client, "Food", "expense").await;
    create_target(&client, &expense_cat, 50_000).await;

    // Create an account and a transaction in the period
    let account_id = create_account(&client, "Checking Main", 100_000).await;
    create_transaction(&client, &account_id, &expense_cat, 15_000, "2026-03-10").await;

    let resp = client
        .get(format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // spent should reflect the transaction we created
    assert_eq!(body["spent"], 15_000);
    // target should reflect the budget category target
    assert_eq!(body["target"], 50_000);
    // daysInPeriod for a 2026-03-01 to 2026-03-31 period = 31 (both endpoints inclusive)
    assert_eq!(body["daysInPeriod"], 31);
    // daysRemaining should be >= 0 and <= 31
    let days_remaining = body["daysRemaining"].as_i64().unwrap();
    assert!((0..=31).contains(&days_remaining), "daysRemaining={days_remaining}");
    // projectedSpend should be a non-negative number (can vary by day)
    let projected_spend = body["projectedSpend"].as_i64().unwrap();
    assert!(projected_spend >= 0, "projectedSpend={projected_spend}");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_ended_period_zero_remaining() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create an ended period (entirely in the past)
    let period_id = create_period(&client, "2026-01-01", "2026-01-31").await;

    let resp = client
        .get(format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Ended period should have 0 remaining days
    assert_eq!(body["daysRemaining"], 0);
    assert_eq!(body["daysInPeriod"], 31);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/current-period", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_nonexistent_period_id_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let fake_id = uuid::Uuid::new_v4();
    let resp = client
        .get(format!("{}/dashboard/current-period?periodId={}", V2_BASE, fake_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/current-period?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/net-position
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_net_position_with_transactions() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create period covering today so transactions land in period_change
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    // Create accounts
    let account_id = create_account(&client, "Net Checking", 0).await;

    // Create income and expense categories
    let income_cat = create_category(&client, "Salary NP", "income").await;
    let expense_cat = create_category(&client, "Food NP", "expense").await;

    // Create transactions within the period
    create_transaction(&client, &account_id, &income_cat, 10_000, "2026-03-10").await;
    create_transaction(&client, &account_id, &expense_cat, 3_000, "2026-03-15").await;

    let resp = client
        .get(format!("{}/dashboard/net-position?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Net position: initial balance (0) + income (10000) - expense (3000) = 7000
    assert_eq!(body["total"], 7_000);
    // Difference this period: income - expense = 10000 - 3000 = 7000
    assert_eq!(body["differenceThisPeriod"], 7_000);
    assert_eq!(body["numberOfAccounts"], 1);
    // Checking account is liquid
    assert_eq!(body["liquidAmount"], 7_000);
    assert_eq!(body["protectedAmount"], 0);
    assert_eq!(body["debtAmount"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_net_position_empty_period_all_zeros() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Period with no accounts or transactions
    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/dashboard/net-position?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["total"], 0);
    assert_eq!(body["differenceThisPeriod"], 0);
    assert_eq!(body["numberOfAccounts"], 0);
    assert_eq!(body["liquidAmount"], 0);
    assert_eq!(body["protectedAmount"], 0);
    assert_eq!(body["debtAmount"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_net_position_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/net-position", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_net_position_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/net-position?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/budget-stability
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_budget_stability_with_closed_periods() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create an expense category and a target
    let expense_cat = create_category(&client, "Food BS", "expense").await;
    create_target(&client, &expense_cat, 10_000).await;

    // Create an account for transactions
    let account_id = create_account(&client, "BS Checking", 100_000).await;

    // Create 2 closed periods (end_date in the past)
    let _period1 = create_period(&client, "2026-01-01", "2026-01-31").await;
    let _period2 = create_period(&client, "2026-02-01", "2026-02-28").await;

    // Create transactions within the closed periods (within tolerance of 10000 target)
    create_transaction(&client, &account_id, &expense_cat, 9_500, "2026-01-15").await;
    create_transaction(&client, &account_id, &expense_cat, 10_200, "2026-02-15").await;

    // Need a current period to pass as periodId
    let current_period = create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/dashboard/budget-stability?periodId={}", V2_BASE, current_period))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // stability should be 0-100
    let stability = body["stability"].as_i64().unwrap();
    assert!((0..=100).contains(&stability), "stability={stability}");

    // periodsWithinRange should be >= 0
    let periods_within = body["periodsWithinRange"].as_i64().unwrap();
    assert!(periods_within >= 0, "periodsWithinRange={periods_within}");

    // periodsStability should be an array of booleans
    let periods_stability = body["periodsStability"].as_array().unwrap();
    assert!(!periods_stability.is_empty(), "periodsStability should not be empty with closed periods");
    for item in periods_stability {
        assert!(item.is_boolean(), "each period stability entry should be a boolean");
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_budget_stability_no_closed_periods_valid_response() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Only a current/future period, no closed ones
    let period_id = create_period(&client, "2026-03-01", "2026-12-31").await;

    let resp = client
        .get(format!("{}/dashboard/budget-stability?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // With no closed periods, stability defaults to 0, periodsWithinRange = 0, empty array
    assert_eq!(body["stability"], 0);
    assert_eq!(body["periodsWithinRange"], 0);
    let periods_stability = body["periodsStability"].as_array().unwrap();
    assert!(periods_stability.is_empty());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_budget_stability_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/budget-stability", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_budget_stability_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/budget-stability?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/current-period — dailySpend sparkline (2.1)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_current_period_daily_spend_sparkline() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-01-01", "2026-01-31").await;
    let expense_cat = create_category(&client, "Food DS", "expense").await;
    let account_id = create_account(&client, "DS Checking", 100_000).await;

    // Create transactions on two different days
    create_transaction(&client, &account_id, &expense_cat, 5_000, "2026-01-05").await;
    create_transaction(&client, &account_id, &expense_cat, 3_000, "2026-01-10").await;

    let resp = client
        .get(format!("{}/dashboard/current-period?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let daily_spend = body["dailySpend"].as_array().unwrap();
    // Jan 1-31 = 31 elements
    assert_eq!(daily_spend.len(), 31, "dailySpend should have one entry per day");

    // Day index 4 (Jan 5) = 5000, index 9 (Jan 10) = 3000
    assert_eq!(daily_spend[4], 5_000);
    assert_eq!(daily_spend[9], 3_000);
    // Day with no transactions should be 0
    assert_eq!(daily_spend[0], 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/budget-stability — recentStability (2.2)
// ═══════════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/cash-flow (2.3)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_cash_flow_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let income_cat = create_category(&client, "Salary CF", "income").await;
    let expense_cat = create_category(&client, "Food CF", "expense").await;
    let account_id = create_account(&client, "CF Checking", 0).await;

    create_transaction(&client, &account_id, &income_cat, 20_000, "2026-03-05").await;
    create_transaction(&client, &account_id, &expense_cat, 7_000, "2026-03-10").await;
    create_transaction(&client, &account_id, &expense_cat, 3_000, "2026-03-15").await;

    let resp = client.get(format!("{}/dashboard/cash-flow?periodId={}", V2_BASE, period_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["inflows"], 20_000);
    assert_eq!(body["outflows"], 10_000);
    assert_eq!(body["net"], 10_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_cash_flow_empty_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client.get(format!("{}/dashboard/cash-flow?periodId={}", V2_BASE, period_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["inflows"], 0);
    assert_eq!(body["outflows"], 0);
    assert_eq!(body["net"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_cash_flow_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/cash-flow", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_cash_flow_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/cash-flow?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/spending-trend (2.4)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_spending_trend_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let expense_cat = create_category(&client, "Food ST", "expense").await;
    let account_id = create_account(&client, "ST Checking", 100_000).await;

    // Two closed periods with known spend
    create_period(&client, "2026-01-01", "2026-01-31").await;
    create_period(&client, "2026-02-01", "2026-02-28").await;
    create_transaction(&client, &account_id, &expense_cat, 8_000, "2026-01-15").await;
    create_transaction(&client, &account_id, &expense_cat, 9_000, "2026-02-15").await;

    let current_period = create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/dashboard/spending-trend?periodId={}", V2_BASE, current_period))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let items = body["periods"].as_array().unwrap();
    assert!(!items.is_empty(), "spending trend should include periods");

    // Each item should have required fields
    for item in items {
        assert!(item["periodId"].is_string());
        assert!(item["periodName"].is_string());
        assert!(item["totalSpent"].is_number());
    }

    // Find Jan and Feb periods by spend amount
    let jan = items.iter().find(|i| i["totalSpent"] == 8_000);
    assert!(jan.is_some(), "should find jan spend of 8000");
    let feb = items.iter().find(|i| i["totalSpent"] == 9_000);
    assert!(feb.is_some(), "should find feb spend of 9000");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_spending_trend_respects_limit() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create 5 periods
    for i in 1..=5_u32 {
        create_period(&client, &format!("2025-{:02}-01", i), &format!("2025-{:02}-28", i)).await;
    }

    let current_period = create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client
        .get(format!("{}/dashboard/spending-trend?periodId={}&limit=3", V2_BASE, current_period))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let items = body["periods"].as_array().unwrap();
    assert_eq!(items.len(), 3, "should return exactly limit=3 items");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_spending_trend_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/spending-trend", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_spending_trend_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/spending-trend?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/top-vendors (2.5)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_top_vendors_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let expense_cat = create_category(&client, "Food TV", "expense").await;
    let account_id = create_account(&client, "TV Checking", 100_000).await;

    let vendor_a = create_vendor(&client, "Vendor Alpha").await;
    let vendor_b = create_vendor(&client, "Vendor Beta").await;

    create_transaction_with_vendor(&client, &account_id, &expense_cat, 10_000, "2026-03-05", &vendor_a).await;
    create_transaction_with_vendor(&client, &account_id, &expense_cat, 4_000, "2026-03-10", &vendor_b).await;

    let resp = client.get(format!("{}/dashboard/top-vendors?periodId={}", V2_BASE, period_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 2);

    // First should be highest spender (Vendor Alpha)
    assert_eq!(items[0]["vendorName"], "Vendor Alpha");
    assert_eq!(items[0]["totalSpent"], 10_000);

    assert_eq!(items[1]["vendorName"], "Vendor Beta");
    assert_eq!(items[1]["totalSpent"], 4_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_top_vendors_empty_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client.get(format!("{}/dashboard/top-vendors?periodId={}", V2_BASE, period_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_top_vendors_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/top-vendors", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_top_vendors_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/top-vendors?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/uncategorized (2.6)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_uncategorized_returns_zero_with_all_categorized() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let expense_cat = create_category(&client, "Food UC", "expense").await;
    let account_id = create_account(&client, "UC Checking", 100_000).await;

    create_transaction(&client, &account_id, &expense_cat, 5_000, "2026-03-10").await;

    let resp = client
        .get(format!("{}/dashboard/uncategorized?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["count"], 0);
    assert_eq!(body["transactions"].as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_uncategorized_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/uncategorized", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_uncategorized_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/uncategorized?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/fixed-categories (2.7)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_fixed_categories_happy() {
    use rocket::http::ContentType;

    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    let account_id = create_account(&client, "FC Checking", 100_000).await;

    // Create a fixed expense category
    let payload = serde_json::json!({
        "name": "Rent FC",
        "type": "expense",
        "behavior": "fixed",
        "icon": "🏠",
        "description": null,
        "parentId": null
    });
    let resp = client
        .post(format!("{}/categories", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);
    let cat_body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let rent_cat_id = cat_body["id"].as_str().unwrap().to_string();

    // Create a variable expense category (should not appear in fixed-categories)
    let expense_cat = create_category(&client, "Groceries FC", "expense").await;

    // Set a budget target for rent
    create_target(&client, &rent_cat_id, 15_000).await;

    // Scenario: rent has no transactions yet → pending
    let resp = client
        .get(format!("{}/dashboard/fixed-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1, "only fixed categories returned");
    assert_eq!(items[0]["categoryName"], "Rent FC");
    assert_eq!(items[0]["status"], "pending");
    assert_eq!(items[0]["spent"], 0);
    assert_eq!(items[0]["budgeted"], 15_000);

    // Add a partial payment (less than target)
    create_transaction(&client, &account_id, &rent_cat_id, 8_000, "2026-03-05").await;

    let resp = client
        .get(format!("{}/dashboard/fixed-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let items = body.as_array().unwrap();
    assert_eq!(items[0]["status"], "partial");
    assert_eq!(items[0]["spent"], 8_000);

    // Pay full amount
    create_transaction(&client, &account_id, &rent_cat_id, 7_000, "2026-03-06").await;

    let resp = client
        .get(format!("{}/dashboard/fixed-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let items = body.as_array().unwrap();
    assert_eq!(items[0]["status"], "paid");
    assert_eq!(items[0]["spent"], 15_000);

    // The variable category should NOT appear
    let _ = expense_cat; // ensure it was created
    for item in items {
        assert_ne!(item["categoryName"], "Groceries FC");
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_fixed_categories_empty_when_no_fixed_categories() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-03-01", "2026-03-31").await;
    // Only variable category — no fixed ones
    create_category(&client, "Food NoFC", "expense").await;

    let resp = client
        .get(format!("{}/dashboard/fixed-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_fixed_categories_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/fixed-categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_fixed_categories_wrapped_happy() {
    use rocket::http::ContentType;

    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-05-01", "2026-05-31").await;
    let account_id = create_account(&client, "FCW Checking", 100_000).await;

    let payload = serde_json::json!({
        "name": "Rent FCW",
        "type": "expense",
        "behavior": "fixed",
        "icon": "🏠",
        "description": null,
        "parentId": null
    });
    let resp = client
        .post(format!("{}/categories", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);
    let cat_body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let rent_cat_id = cat_body["id"].as_str().unwrap().to_string();

    create_target(&client, &rent_cat_id, 15_000).await;
    create_transaction(&client, &account_id, &rent_cat_id, 8_000, "2026-05-05").await;

    let resp = client
        .get(format!("{}/dashboard/fixed-categories?periodId={}&responseFormat=wrapped", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["totalBudgeted"], 15_000);
    assert_eq!(body["totalPaid"], 8_000);
    let categories = body["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0]["name"], "Rent FCW");
    assert_eq!(categories[0]["budgeted"], 15_000);
    assert_eq!(categories[0]["paid"], 8_000);
    assert_eq!(categories[0]["status"], "partial");
    assert!(categories[0]["id"].is_string());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_fixed_categories_wrapped_empty_when_no_fixed_categories() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-05-01", "2026-05-31").await;
    create_category(&client, "Food NoFCW", "expense").await;

    let resp = client
        .get(format!("{}/dashboard/fixed-categories?periodId={}&responseFormat=wrapped", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["totalBudgeted"], 0);
    assert_eq!(body["totalPaid"], 0);
    assert_eq!(body["categories"].as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_fixed_categories_unknown_format_returns_legacy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-05-01", "2026-05-31").await;

    let resp = client
        .get(format!("{}/dashboard/fixed-categories?periodId={}&responseFormat=garbage", V2_BASE, period_id))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body.is_array(), "unknown responseFormat must fall through to the legacy flat array");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_fixed_categories_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/fixed-categories?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /dashboard/variable-categories
// ═══════════════════════════════════════════════════════════════════════════════

async fn create_variable_category(client: &rocket::local::asynchronous::Client, name: &str) -> String {
    use rocket::http::ContentType;
    let payload = serde_json::json!({
        "name": name,
        "type": "expense",
        "behavior": "variable",
        "icon": "🛒",
        "description": null,
        "parentId": null
    });
    let resp = client
        .post(format!("{}/categories", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    body["id"].as_str().unwrap().to_string()
}

async fn create_fixed_category(client: &rocket::local::asynchronous::Client, name: &str) -> String {
    use rocket::http::ContentType;
    let payload = serde_json::json!({
        "name": name,
        "type": "expense",
        "behavior": "fixed",
        "icon": "🏠",
        "description": null,
        "parentId": null
    });
    let resp = client
        .post(format!("{}/categories", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    body["id"].as_str().unwrap().to_string()
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_returns_wrapped_response() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "VC Checking", 100_000).await;

    // Two variable categories with budgets and partial spending
    let groceries = create_variable_category(&client, "Groceries VC").await;
    let dining = create_variable_category(&client, "Dining VC").await;

    create_target(&client, &groceries, 10_000).await;
    create_target(&client, &dining, 5_000).await;

    create_transaction(&client, &account_id, &groceries, 4_000, "2026-04-05").await;
    create_transaction(&client, &account_id, &dining, 2_500, "2026-04-10").await;

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Wrapped object per OpenAPI spec
    assert_eq!(body["totalBudgeted"], 15_000);
    assert_eq!(body["totalPaid"], 6_500);

    let categories = body["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 2);

    // Sorted alphabetically: Dining, Groceries
    assert_eq!(categories[0]["name"], "Dining VC");
    assert_eq!(categories[0]["budgeted"], 5_000);
    assert_eq!(categories[0]["paid"], 2_500);
    assert_eq!(categories[0]["progress"], 50);
    assert!(categories[0]["id"].is_string());

    assert_eq!(categories[1]["name"], "Groceries VC");
    assert_eq!(categories[1]["budgeted"], 10_000);
    assert_eq!(categories[1]["paid"], 4_000);
    assert_eq!(categories[1]["progress"], 40);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_excludes_fixed_categories() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "VC Excl Acct", 100_000).await;

    let variable = create_variable_category(&client, "VarCat Excl").await;
    let fixed = create_fixed_category(&client, "FixCat Excl").await;

    create_target(&client, &variable, 5_000).await;
    create_target(&client, &fixed, 8_000).await;
    create_transaction(&client, &account_id, &variable, 1_000, "2026-04-05").await;
    create_transaction(&client, &account_id, &fixed, 4_000, "2026-04-06").await;

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let categories = body["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0]["name"], "VarCat Excl");
    assert_eq!(body["totalBudgeted"], 5_000);
    assert_eq!(body["totalPaid"], 1_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_excludes_income_categories() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let _account_id = create_account(&client, "VC Income Acct", 100_000).await;

    create_variable_category(&client, "Salary VC").await; // would be expense, name only
    // Create an income category — should be excluded regardless of behavior
    create_category(&client, "RealSalary", "income").await;

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let categories = body["categories"].as_array().unwrap();
    let names: Vec<&str> = categories.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"Salary VC"));
    assert!(!names.contains(&"RealSalary"));
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_progress_clamps_at_100_when_overspent() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "VC Over Acct", 100_000).await;

    let cat = create_variable_category(&client, "Overspent VC").await;
    create_target(&client, &cat, 5_000).await;
    create_transaction(&client, &account_id, &cat, 8_000, "2026-04-05").await;

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let categories = body["categories"].as_array().unwrap();
    assert_eq!(categories[0]["paid"], 8_000);
    assert_eq!(categories[0]["budgeted"], 5_000);
    assert_eq!(categories[0]["progress"], 100, "progress clamps at 100");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_progress_zero_when_no_budget() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "VC NoBudget Acct", 100_000).await;

    let cat = create_variable_category(&client, "NoBudget VC").await;
    create_transaction(&client, &account_id, &cat, 3_000, "2026-04-05").await;

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let categories = body["categories"].as_array().unwrap();
    assert_eq!(categories[0]["budgeted"], 0);
    assert_eq!(categories[0]["paid"], 3_000);
    assert_eq!(categories[0]["progress"], 0, "progress is 0 when no budget");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_excludes_transactions_outside_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "VC Window Acct", 100_000).await;

    let cat = create_variable_category(&client, "Windowed VC").await;
    create_target(&client, &cat, 10_000).await;

    create_transaction(&client, &account_id, &cat, 1_000, "2026-03-31").await; // before
    create_transaction(&client, &account_id, &cat, 2_000, "2026-04-15").await; // inside
    create_transaction(&client, &account_id, &cat, 3_000, "2026-05-01").await; // after

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let categories = body["categories"].as_array().unwrap();
    assert_eq!(categories[0]["paid"], 2_000, "only in-period transactions count");
    assert_eq!(body["totalPaid"], 2_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_empty_when_no_variable_categories() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    create_fixed_category(&client, "OnlyFixed VC").await;

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["totalBudgeted"], 0);
    assert_eq!(body["totalPaid"], 0);
    assert_eq!(body["categories"].as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_missing_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/dashboard/variable-categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_invalid_period_id_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId=not-a-uuid", V2_BASE))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, uuid::Uuid::new_v4()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_variable_categories_user_isolation() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = create_period(&client, "2026-04-01", "2026-04-30").await;
    let account_id = create_account(&client, "Iso VC Acct", 100_000).await;
    let cat = create_variable_category(&client, "Iso VC").await;
    create_target(&client, &cat, 5_000).await;
    create_transaction(&client, &account_id, &cat, 2_000, "2026-04-05").await;

    // Switch to a different user
    create_user_and_login(&client).await;

    // The second user should not see the first user's variable categories
    let resp = client
        .get(format!("{}/dashboard/variable-categories?periodId={}", V2_BASE, period_id))
        .dispatch()
        .await;
    // The period belongs to the first user; the second user should get a 404
    // (period not found) — matching the fixed-categories behavior.
    assert_eq!(resp.status(), Status::NotFound);
}
