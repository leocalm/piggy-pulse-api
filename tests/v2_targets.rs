mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /targets (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/targets", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_targets_with_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client.get(format!("{}/targets?periodId={}", V2_BASE, period_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["summary"].is_object());
    assert!(body["targets"].is_array());
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
async fn test_create_target_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let category_id = common::entities::create_category(&client, "Target Cat", "expense").await;

    let payload = serde_json::json!({
        "categoryId": category_id,
        "value": 50000
    });

    let resp = client
        .post(format!("{}/targets", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_target_negative_value() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let category_id = common::entities::create_category(&client, "Neg Target Cat", "expense").await;

    let payload = serde_json::json!({
        "categoryId": category_id,
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
async fn test_create_target_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "categoryId": Uuid::new_v4(),
        "value": 50000
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
async fn test_update_target_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "categoryId": Uuid::new_v4(),
        "value": 75000
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
        .body("{}")
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /targets/{id}/exclude
// ═══════════════════════════════════════════════════════════════════════════════

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
