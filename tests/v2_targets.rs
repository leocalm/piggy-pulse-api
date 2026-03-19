mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /targets (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_with_period_reflects_data() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "TargetListCat", "expense").await;
    let account_id = common::entities::create_account(&client, "TargetListAcct", 500_000).await;
    let period_id = common::entities::create_period(&client, "2026-04-01", "2026-04-30").await;

    common::entities::create_target(&client, &cat_id, 50_000).await;
    common::entities::create_transaction(&client, &account_id, &cat_id, 20_000, "2026-04-10").await;

    let resp = client.get(format!("{}/targets?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert!(body["summary"].is_object());
    let targets = body["targets"].as_array().unwrap();
    let target = targets
        .iter()
        .find(|t| t["name"].as_str().unwrap() == "TargetListCat")
        .expect("target for TargetListCat should appear");
    assert_eq!(target["spentInPeriod"], 20_000);
    assert_eq!(target["currentTarget"], 50_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_previous_target_null_for_first_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "FirstPeriodCat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-07-01", "2026-07-31").await;

    common::entities::create_target(&client, &cat_id, 30_000).await;

    let resp = client.get(format!("{}/targets?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let targets = body["targets"].as_array().unwrap();
    let target = targets.iter().find(|t| t["name"].as_str().unwrap() == "FirstPeriodCat").unwrap();
    assert!(target["previousTarget"].is_null(), "previousTarget should be null for first period");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_missing_period_id() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/targets", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_empty_state() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-08-01", "2026-08-31").await;

    let resp = client.get(format!("{}/targets?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["summary"].is_object());
    let targets = body["targets"].as_array().unwrap();
    assert!(targets.is_empty(), "no targets should exist for fresh user/period");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/targets", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /targets (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_target_returns_correct_value() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "NewTargetCat", "expense").await;

    let payload = json!({
        "categoryId": cat_id,
        "value": 30_000
    });

    let resp = client
        .post(format!("{}/targets", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["currentTarget"], 30_000);
    assert_eq!(body["status"], "active");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_target_duplicate_conflict() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "DupTargetCat", "expense").await;
    common::entities::create_target(&client, &cat_id, 10_000).await;

    // Creating a second target for the same category should conflict
    let payload = json!({
        "categoryId": cat_id,
        "value": 20_000
    });

    let resp = client
        .post(format!("{}/targets", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_target_nonexistent_category() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "categoryId": Uuid::new_v4(),
        "value": 10_000
    });

    let resp = client
        .post(format!("{}/targets", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::NotFound || resp.status() == Status::BadRequest,
        "expected 404 or 400 for nonexistent category, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_target_no_auth() {
    let client = test_client().await;

    let payload = json!({
        "categoryId": Uuid::new_v4(),
        "value": 50_000
    });

    let resp = client
        .post(format!("{}/targets", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /targets/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_target_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "UpdateTargetCat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-09-01", "2026-09-30").await;
    let target_id = common::entities::create_target(&client, &cat_id, 30_000).await;

    let payload = json!({
        "categoryId": cat_id,
        "value": 50_000
    });

    let resp = client
        .put(format!("{}/targets/{}", V2_BASE, target_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let put_body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(put_body["currentTarget"], 50_000);

    // Verify via GET /targets — persistence, not just echo
    let list_resp = client.get(format!("{}/targets?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let targets = list_body["targets"].as_array().unwrap();
    let target = targets
        .iter()
        .find(|t| t["id"].as_str().unwrap() == target_id)
        .expect("updated target should appear in list");
    assert_eq!(target["currentTarget"], 50_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_target_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "categoryId": Uuid::new_v4(),
        "value": 75_000
    });

    let resp = client
        .put(format!("{}/targets/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_target_no_auth() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/targets/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(json!({"categoryId": Uuid::new_v4(), "value": 1000}).to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /targets/{id}/exclude
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_target_sets_excluded_status() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "ExcludeCat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-10-01", "2026-10-31").await;
    let target_id = common::entities::create_target(&client, &cat_id, 25_000).await;

    let resp = client.post(format!("{}/targets/{}/exclude", V2_BASE, target_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "excluded");

    // Verify via GET /targets
    let list_resp = client.get(format!("{}/targets?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let targets = list_body["targets"].as_array().unwrap();
    let target = targets
        .iter()
        .find(|t| t["id"].as_str().unwrap() == target_id)
        .expect("excluded target should still appear in list");
    assert_eq!(target["status"], "excluded");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_already_excluded_conflict() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "DoubleExcludeCat", "expense").await;
    let target_id = common::entities::create_target(&client, &cat_id, 10_000).await;

    let first = client.post(format!("{}/targets/{}/exclude", V2_BASE, target_id)).dispatch().await;
    assert_eq!(first.status(), Status::Ok);

    let second = client.post(format!("{}/targets/{}/exclude", V2_BASE, target_id)).dispatch().await;
    assert_eq!(second.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_target_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/targets/{}/exclude", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_target_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/targets/{}/exclude", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// User isolation
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_targets_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let cat_id = common::entities::create_category(&client_a, "User A TargetCat", "expense").await;
    common::entities::create_target(&client_a, &cat_id, 50_000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;
    let period_b = common::entities::create_period(&client_b, "2026-11-01", "2026-11-30").await;

    let resp = client_b.get(format!("{}/targets?periodId={}", V2_BASE, period_b)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["targets"].as_array().unwrap().is_empty(), "User B should see no targets from User A");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_target_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let cat_id = common::entities::create_category(&client_a, "Isolated Target", "expense").await;
    let target_id = common::entities::create_target(&client_a, &cat_id, 30_000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let payload = json!({
        "categoryId": cat_id,
        "value": 99_999
    });

    let resp = client_b
        .put(format!("{}/targets/{}", V2_BASE, target_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_target_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let cat_id = common::entities::create_category(&client_a, "Exclude Isolated", "expense").await;
    let target_id = common::entities::create_target(&client_a, &cat_id, 20_000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.post(format!("{}/targets/{}/exclude", V2_BASE, target_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}
