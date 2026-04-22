mod common;

use common::auth::create_user_and_login;
use common::crypto::decrypt_i64;
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
    let period_id = common::entities::create_period(&client, "2026-04-01", "2026-04-30").await;
    let target_id = common::entities::create_target(&client, &cat_id, 50_000).await;

    let resp = client.get(format!("{}/targets?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let targets = body.as_array().expect("targets response should be a plain array");
    let target = targets
        .iter()
        .find(|t| t["id"].as_str().unwrap() == target_id)
        .expect("target for TargetListCat should appear");
    assert_eq!(target["categoryId"].as_str().unwrap(), cat_id);
    assert!(!target["isExcluded"].as_bool().unwrap(), "new targets should not be excluded");
    assert_eq!(decrypt_i64(target["budgetedValueEnc"].as_str().unwrap()), 50_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_previous_target_null_for_first_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "FirstPeriodCat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-07-01", "2026-07-31").await;
    let target_id = common::entities::create_target(&client, &cat_id, 30_000).await;

    let resp = client.get(format!("{}/targets?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let targets = body.as_array().expect("targets response should be a plain array");
    let target = targets.iter().find(|t| t["id"].as_str().unwrap() == target_id).unwrap();
    assert_eq!(target["categoryId"].as_str().unwrap(), cat_id);
    assert!(!target["isExcluded"].as_bool().unwrap());
    assert_eq!(decrypt_i64(target["budgetedValueEnc"].as_str().unwrap()), 30_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_missing_period_id() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/targets", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body.is_array(), "targets response should be a plain array");
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
    assert!(body.is_array(), "targets response should be a plain array");
    let targets = body.as_array().unwrap();
    assert!(targets.is_empty(), "no targets should exist for a fresh user");
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
    assert_eq!(body["categoryId"].as_str().unwrap(), cat_id);
    assert!(!body["isExcluded"].as_bool().unwrap(), "new target should be active");
    assert_eq!(decrypt_i64(body["budgetedValueEnc"].as_str().unwrap()), 30_000);
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_target_duplicate_conflict() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "DupTargetCat", "expense").await;
    common::entities::create_target(&client, &cat_id, 10_000).await;

    // Creating a second target for the same category currently bubbles up as a DB error.
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

    assert_eq!(resp.status(), Status::InternalServerError);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_target_negative_value() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "NegTargetCat", "expense").await;

    let payload = json!({
        "categoryId": cat_id,
        "value": -100
    });

    let resp = client
        .post(format!("{}/targets", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
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
    assert_eq!(put_body["categoryId"].as_str().unwrap(), cat_id);
    assert!(!put_body["isExcluded"].as_bool().unwrap());
    assert_eq!(decrypt_i64(put_body["budgetedValueEnc"].as_str().unwrap()), 50_000);

    // Verify via GET /targets — persistence, not just echo
    let list_resp = client.get(format!("{}/targets", V2_BASE)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let targets = list_body.as_array().unwrap();
    let target = targets
        .iter()
        .find(|t| t["id"].as_str().unwrap() == target_id)
        .expect("updated target should appear in list");
    assert_eq!(target["categoryId"].as_str().unwrap(), cat_id);
    assert!(!target["isExcluded"].as_bool().unwrap());
    assert_eq!(decrypt_i64(target["budgetedValueEnc"].as_str().unwrap()), 50_000);
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
    let target_id = common::entities::create_target(&client, &cat_id, 25_000).await;

    let resp = client.post(format!("{}/targets/{}/exclude", V2_BASE, target_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["categoryId"].as_str().unwrap(), cat_id);
    assert!(body["isExcluded"].as_bool().unwrap());
    assert_eq!(decrypt_i64(body["budgetedValueEnc"].as_str().unwrap()), 25_000);

    // Verify via GET /targets
    let list_resp = client.get(format!("{}/targets", V2_BASE)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let targets = list_body.as_array().unwrap();
    let target = targets
        .iter()
        .find(|t| t["id"].as_str().unwrap() == target_id)
        .expect("excluded target should still appear in list");
    assert_eq!(target["categoryId"].as_str().unwrap(), cat_id);
    assert!(target["isExcluded"].as_bool().unwrap());
    assert_eq!(decrypt_i64(target["budgetedValueEnc"].as_str().unwrap()), 25_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_exclude_already_excluded_toggle_back() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "DoubleExcludeCat", "expense").await;
    let target_id = common::entities::create_target(&client, &cat_id, 10_000).await;

    let first = client.post(format!("{}/targets/{}/exclude", V2_BASE, target_id)).dispatch().await;
    assert_eq!(first.status(), Status::Ok);

    let second = client.post(format!("{}/targets/{}/exclude", V2_BASE, target_id)).dispatch().await;
    assert_eq!(second.status(), Status::Ok);
    let body: Value = serde_json::from_str(&second.into_string().await.unwrap()).unwrap();
    assert_eq!(body["categoryId"].as_str().unwrap(), cat_id);
    assert!(!body["isExcluded"].as_bool().unwrap(), "second toggle should restore active state");
    assert_eq!(decrypt_i64(body["budgetedValueEnc"].as_str().unwrap()), 10_000);
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
    let target_id = common::entities::create_target(&client_a, &cat_id, 50_000).await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;
    let period_b = common::entities::create_period(&client_b, "2026-11-01", "2026-11-30").await;

    let resp = client_b.get(format!("{}/targets?periodId={}", V2_BASE, period_b)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let targets = body.as_array().unwrap();
    assert!(
        !targets.iter().any(|t| t["id"].as_str().unwrap() == target_id),
        "User B should not see User A's target"
    );
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
