mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_period, create_target, create_transaction};
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
    // daysInPeriod for a 2026-03-01 to 2026-03-31 period = 30
    assert_eq!(body["daysInPeriod"], 30);
    // daysRemaining should be >= 0 and <= 30
    let days_remaining = body["daysRemaining"].as_i64().unwrap();
    assert!((0..=30).contains(&days_remaining), "daysRemaining={days_remaining}");
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
    assert_eq!(body["daysInPeriod"], 30);
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
