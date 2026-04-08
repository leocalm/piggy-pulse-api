mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category};
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

/// Helper: create a manual period schedule via POST /periods/schedule
async fn create_period_schedule(client: &rocket::local::asynchronous::Client) {
    let payload = serde_json::json!({
        "scheduleType": "manual"
    });
    let resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "create_period_schedule failed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /onboarding/status
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_status_fresh_user_no_currency_is_not_started() {
    let client = test_client().await;

    // Register directly without setting currency (currency is the first required step)
    let email = format!("fresh.{}@example.com", Uuid::new_v4());
    let register_payload = serde_json::json!({
        "name": "Fresh User",
        "email": email,
        "password": common::TEST_PASSWORD,
    });
    let resp = client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(register_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);

    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    // No currency set → status is not_started, currentStep is "currency"
    assert_eq!(body["status"], "not_started");
    assert_eq!(body["currentStep"], "currency");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_status_after_currency_set_is_in_progress() {
    let client = test_client().await;
    // create_user_and_login registers and sets currency (EUR)
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    // Currency set, no period schedule → in_progress at "period" step
    assert_eq!(body["status"], "in_progress");
    assert_eq!(body["currentStep"], "period");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_status_current_step_is_valid_or_null() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let valid_steps = ["currency", "period", "accounts", "categories", "summary"];

    if !body["currentStep"].is_null() {
        let step = body["currentStep"].as_str().expect("currentStep must be string or null");
        assert!(valid_steps.contains(&step), "currentStep must be one of {:?}, got {}", valid_steps, step);
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_status_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_status_in_progress_after_period_schedule_created() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create a period schedule — this moves us past the "period" step
    create_period_schedule(&client).await;

    // Status should now be in_progress — the next step after period schedule depends on flow config
    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "in_progress");
    // After period schedule, the next step may be "accounts" or "summary" depending on
    // whether accounts/categories steps are optional
    let step = body["currentStep"].as_str().unwrap();
    assert!(
        step == "accounts" || step == "summary",
        "expected currentStep to be 'accounts' or 'summary', got '{step}'"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /onboarding/complete
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_complete_returns_204_and_status_completed() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Set up all onboarding prerequisites:
    // 1. Period schedule
    create_period_schedule(&client).await;

    // 2. Account
    create_account(&client, "Onboarding Account", 50_000).await;

    // 3. Categories (both income and expense — V2 uses lowercase)
    create_category(&client, "Salary Income", "income").await;
    create_category(&client, "Food Expense", "expense").await;

    // Verify we're at the summary step
    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "in_progress");
    assert_eq!(body["currentStep"], "summary");

    // Complete onboarding
    let resp = client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify status is now completed via subsequent GET
    let resp = client.get(format!("{}/onboarding/status", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "completed");
    assert!(body["currentStep"].is_null(), "currentStep must be null after completion");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_complete_already_completed_is_idempotent() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Set up all prerequisites
    create_period_schedule(&client).await;
    create_account(&client, "Idem Account", 10_000).await;
    create_category(&client, "Idem Income", "income").await;
    create_category(&client, "Idem Expense", "expense").await;

    // Complete once
    let resp = client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Complete again — should be idempotent (204)
    let resp = client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_complete_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_onboarding_complete_without_prerequisites_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Try to complete without any onboarding steps done
    let resp = client.post(format!("{}/onboarding/complete", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}
