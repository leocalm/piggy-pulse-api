mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /periods (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_manual_end_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-03-01",
        "name": "March 2026",
        "manualEndDate": "2026-03-31"
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "March 2026");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_duration_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "Duration",
        "startDate": "2026-04-01",
        "name": "April 2026",
        "duration": {
            "durationUnits": 1,
            "durationUnit": "months"
        }
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["periodType"], "Duration");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_missing_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "name": "Incomplete"
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400 or 422, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-03-01",
        "name": "No Auth",
        "manualEndDate": "2026-03-31"
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /periods (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_periods_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;

    let resp = client.get(format!("{}/periods", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_periods_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/periods", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /periods/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_period_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-05-01", "2026-05-31").await;

    let resp = client.get(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["id"], period_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_period_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/periods/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_period_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/periods/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /periods/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_period_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-06-01", "2026-06-30").await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-06-01",
        "name": "Updated June",
        "manualEndDate": "2026-06-28"
    });

    let resp = client
        .put(format!("{}/periods/{}", V2_BASE, period_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "Updated June");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_period_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-06-01",
        "name": "Ghost",
        "manualEndDate": "2026-06-30"
    });

    let resp = client
        .put(format!("{}/periods/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_period_no_auth() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/periods/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body("{}")
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /periods/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_period_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let period_id = common::entities::create_period(&client, "2026-07-01", "2026-07-31").await;

    let resp = client.delete(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NoContent);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_period_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/periods/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_period_no_auth() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/periods/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Period Schedule (GET, POST, PUT, DELETE /periods/schedule)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_schedule_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/periods/schedule", V2_BASE)).dispatch().await;

    // May return 200 (with schedule) or 404 (no schedule yet)
    assert!(
        resp.status() == Status::Ok || resp.status() == Status::NotFound,
        "expected 200 or 404, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_schedule_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/periods/schedule", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_schedule_manual_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "scheduleType": "Manual"
    });

    let resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_schedule_automatic_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "scheduleType": "Automatic",
        "startDayOfTheMonth": 1,
        "periodDuration": 1,
        "generateAhead": 3,
        "durationUnit": "months",
        "saturdayPolicy": "keep",
        "sundayPolicy": "keep",
        "namePattern": "Budget {month} {year}"
    });

    let resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_schedule_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "scheduleType": "Manual"
    });

    let resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_schedule_no_auth() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/periods/schedule", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
