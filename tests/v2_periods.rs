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
async fn test_create_period_duration_asserts_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "Duration",
        "startDate": "2026-04-01",
        "name": "April Duration",
        "duration": {
            "durationUnits": 30,
            "durationUnit": "days"
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

    common::assertions::assert_uuid(&body["id"]);
    assert_eq!(body["name"], "April Duration");
    assert_eq!(body["startDate"], "2026-04-01");
    assert_eq!(body["periodType"], "duration");
    assert_eq!(body["length"], 30);
    assert_eq!(body["numberOfTransactions"], 0);
    assert_eq!(body["status"], "upcoming");
    // Duration-based fields
    assert_eq!(body["duration"]["durationUnits"], 30);
    assert_eq!(body["duration"]["durationUnit"], "days");
    // Upcoming period has remainingDays > 0
    assert!(body["remainingDays"].as_i64().unwrap() > 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_manual_end_date_asserts_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-05-01",
        "name": "May Manual",
        "manualEndDate": "2026-05-31"
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    common::assertions::assert_uuid(&body["id"]);
    assert_eq!(body["name"], "May Manual");
    assert_eq!(body["startDate"], "2026-05-01");
    assert_eq!(body["periodType"], "manualEndDate");
    assert_eq!(body["manualEndDate"], "2026-05-31");
    assert_eq!(body["length"], 30); // May 1 to May 31 = 30 days
    assert_eq!(body["numberOfTransactions"], 0);
    assert_eq!(body["status"], "upcoming");
    assert!(body["remainingDays"].as_i64().unwrap() > 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_past_has_null_remaining_days() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2024-01-01",
        "name": "Past Period",
        "manualEndDate": "2024-01-31"
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["status"], "past");
    assert!(body["remainingDays"].is_null(), "remainingDays should be null for past periods");
    assert_eq!(body["length"], 30);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_active_has_remaining_days() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Use today as start and a far future end date
    let today = chrono::Utc::now().date_naive();
    let end = today + chrono::Duration::days(60);
    let start_str = today.format("%Y-%m-%d").to_string();
    let end_str = end.format("%Y-%m-%d").to_string();

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": start_str,
        "name": "Active Period",
        "manualEndDate": end_str
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["status"], "active");
    let remaining = body["remainingDays"].as_i64().unwrap();
    assert!(remaining > 0, "remainingDays should be > 0 for active periods, got {}", remaining);
    assert_eq!(remaining, 60); // end is 60 days from start=today
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_future_has_upcoming_status() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2027-06-01",
        "name": "Future Period",
        "manualEndDate": "2027-06-30"
    });

    let resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["status"], "upcoming");
    assert!(body["remainingDays"].as_i64().unwrap() > 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_period_missing_fields_returns_400() {
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
async fn test_create_period_no_auth_returns_401() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-03-01",
        "name": "No Auth Period",
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
async fn test_list_periods_returns_created_periods() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let id1 = common::entities::create_period(&client, "2026-08-01", "2026-08-31").await;
    let id2 = common::entities::create_period(&client, "2026-09-01", "2026-09-30").await;

    let resp = client.get(format!("{}/periods", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    common::assertions::assert_paginated(&body);
    let data = body["data"].as_array().unwrap();
    assert!(data.len() >= 2, "expected at least 2 periods, got {}", data.len());

    let ids: Vec<&str> = data.iter().map(|d| d["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&id1.as_str()), "id1 not found in list");
    assert!(ids.contains(&id2.as_str()), "id2 not found in list");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_periods_pagination() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    common::entities::create_period(&client, "2026-10-01", "2026-10-31").await;
    common::entities::create_period(&client, "2026-11-01", "2026-11-30").await;
    common::entities::create_period(&client, "2026-12-01", "2026-12-31").await;

    let resp = client.get(format!("{}/periods?limit=1", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["hasMore"], true);
    assert!(body["totalCount"].as_i64().unwrap() >= 3);
    assert!(body["nextCursor"].is_string());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_periods_total_count_matches() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    common::entities::create_period(&client, "2027-01-01", "2027-01-31").await;
    common::entities::create_period(&client, "2027-02-01", "2027-02-28").await;

    let resp = client.get(format!("{}/periods", V2_BASE)).dispatch().await;
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let total_count = body["totalCount"].as_i64().unwrap();
    let data_len = body["data"].as_array().unwrap().len() as i64;
    assert_eq!(total_count, data_len, "totalCount should match data length when no pagination");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_periods_no_auth_returns_401() {
    let client = test_client().await;
    let resp = client.get(format!("{}/periods", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /periods/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_period_all_fields_match() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create via POST
    let create_payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-06-01",
        "name": "June Get Test",
        "manualEndDate": "2026-06-30"
    });
    let create_resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(create_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(create_resp.status(), Status::Created);
    let created: Value = serde_json::from_str(&create_resp.into_string().await.unwrap()).unwrap();
    let period_id = created["id"].as_str().unwrap();

    // GET and compare
    let get_resp = client.get(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["id"], period_id);
    assert_eq!(body["name"], "June Get Test");
    assert_eq!(body["startDate"], "2026-06-01");
    assert_eq!(body["length"], 29); // Jun 1 to Jun 30 = 29 days
    assert_eq!(body["numberOfTransactions"], 0);
    assert_eq!(body["periodType"], "manualEndDate");
    assert_eq!(body["manualEndDate"], "2026-06-30");
    assert!(body["status"].is_string());
    common::assertions::assert_uuid(&body["id"]);
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
async fn test_get_period_no_auth_returns_401() {
    let client = test_client().await;
    let resp = client.get(format!("{}/periods/{}", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /periods/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_period_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = common::entities::create_period(&client, "2026-07-01", "2026-07-31").await;

    let update_payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-07-01",
        "name": "Updated July",
        "manualEndDate": "2026-07-28"
    });

    let put_resp = client
        .put(format!("{}/periods/{}", V2_BASE, period_id))
        .header(ContentType::JSON)
        .body(update_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(put_resp.status(), Status::Ok);

    // Verify via GET
    let get_resp = client.get(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["name"], "Updated July");
    assert_eq!(body["manualEndDate"], "2026-07-28");
    assert_eq!(body["length"], 27); // Jul 1 to Jul 28 = 27 days
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_period_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "ManualEndDate",
        "startDate": "2026-07-01",
        "name": "Ghost Update",
        "manualEndDate": "2026-07-31"
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
async fn test_update_period_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/periods/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "periodType": "ManualEndDate",
                "startDate": "2026-07-01",
                "name": "No Auth",
                "manualEndDate": "2026-07-31"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /periods/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_period_then_get_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = common::entities::create_period(&client, "2026-08-01", "2026-08-31").await;

    let del_resp = client.delete(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(del_resp.status(), Status::NoContent);

    let get_resp = client.get(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(get_resp.status(), Status::NotFound);
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
async fn test_delete_period_no_auth_returns_401() {
    let client = test_client().await;
    let resp = client.delete(format!("{}/periods/{}", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /periods/schedule
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_schedule_404_when_none_exists() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/periods/schedule", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_schedule_after_create_matches() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create automatic schedule
    let create_payload = serde_json::json!({
        "scheduleType": "automatic",
        "startDayOfTheMonth": 15,
        "periodDuration": 7,
        "generateAhead": 5,
        "durationUnit": "days",
        "saturdayPolicy": "friday",
        "sundayPolicy": "monday",
        "namePattern": "Week {start_date}"
    });

    let create_resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(create_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(create_resp.status(), Status::Created);

    // GET
    let get_resp = client.get(format!("{}/periods/schedule", V2_BASE)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["scheduleType"], "automatic");
    assert_eq!(body["startDayOfTheMonth"], 15);
    assert_eq!(body["periodDuration"], 7);
    assert_eq!(body["generateAhead"], 5);
    assert_eq!(body["durationUnit"], "days");
    assert_eq!(body["saturdayPolicy"], "friday");
    assert_eq!(body["sundayPolicy"], "monday");
    assert_eq!(body["namePattern"], "Week {start_date}");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_schedule_no_auth_returns_401() {
    let client = test_client().await;
    let resp = client.get(format!("{}/periods/schedule", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /periods/schedule
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_schedule_automatic_asserts_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "scheduleType": "automatic",
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
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["scheduleType"], "automatic");
    assert_eq!(body["startDayOfTheMonth"], 1);
    assert_eq!(body["periodDuration"], 1);
    assert_eq!(body["generateAhead"], 3);
    assert_eq!(body["durationUnit"], "months");
    assert_eq!(body["saturdayPolicy"], "keep");
    assert_eq!(body["sundayPolicy"], "keep");
    assert_eq!(body["namePattern"], "Budget {month} {year}");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_schedule_manual_asserts_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "scheduleType": "manual"
    });

    let resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["scheduleType"], "manual");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_schedule_conflict_409() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "scheduleType": "manual"
    });

    // First create
    let resp1 = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp1.status(), Status::Created);

    // Second create should conflict
    let resp2 = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp2.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_schedule_missing_fields_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Automatic schedule missing required fields
    let payload = serde_json::json!({
        "scheduleType": "automatic",
        "startDayOfTheMonth": 1
        // missing periodDuration, generateAhead, durationUnit, etc.
    });

    let resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
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
async fn test_create_schedule_no_auth_returns_401() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "scheduleType": "manual"
    });

    let resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /periods/schedule
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_schedule_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create initial schedule
    let create_payload = serde_json::json!({
        "scheduleType": "automatic",
        "startDayOfTheMonth": 1,
        "periodDuration": 1,
        "generateAhead": 2,
        "durationUnit": "months",
        "saturdayPolicy": "keep",
        "sundayPolicy": "keep",
        "namePattern": "Original {month}"
    });

    let create_resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(create_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(create_resp.status(), Status::Created);

    // Update to different values
    let update_payload = serde_json::json!({
        "scheduleType": "automatic",
        "startDayOfTheMonth": 10,
        "periodDuration": 14,
        "generateAhead": 4,
        "durationUnit": "days",
        "saturdayPolicy": "friday",
        "sundayPolicy": "monday",
        "namePattern": "Updated {start_date}"
    });

    let put_resp = client
        .put(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(update_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(put_resp.status(), Status::Ok);

    // Verify via GET
    let get_resp = client.get(format!("{}/periods/schedule", V2_BASE)).dispatch().await;
    assert_eq!(get_resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&get_resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["scheduleType"], "automatic");
    assert_eq!(body["startDayOfTheMonth"], 10);
    assert_eq!(body["periodDuration"], 14);
    assert_eq!(body["generateAhead"], 4);
    assert_eq!(body["durationUnit"], "days");
    assert_eq!(body["saturdayPolicy"], "friday");
    assert_eq!(body["sundayPolicy"], "monday");
    assert_eq!(body["namePattern"], "Updated {start_date}");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_schedule_404_when_none_exists() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "scheduleType": "manual"
    });

    let resp = client
        .put(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_schedule_no_auth_returns_401() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(serde_json::json!({"scheduleType": "manual"}).to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /periods/schedule
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_schedule_then_get_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create schedule
    let payload = serde_json::json!({
        "scheduleType": "manual"
    });
    let create_resp = client
        .post(format!("{}/periods/schedule", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(create_resp.status(), Status::Created);

    // Delete
    let del_resp = client.delete(format!("{}/periods/schedule", V2_BASE)).dispatch().await;
    assert_eq!(del_resp.status(), Status::NoContent);

    // GET should 404
    let get_resp = client.get(format!("{}/periods/schedule", V2_BASE)).dispatch().await;
    assert_eq!(get_resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_schedule_404_when_none_exists() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/periods/schedule", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_schedule_no_auth_returns_401() {
    let client = test_client().await;
    let resp = client.delete(format!("{}/periods/schedule", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_remaining_days_null_for_ended_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let period_id = common::entities::create_period(&client, "2024-06-01", "2024-06-30").await;

    let resp = client.get(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["status"], "past");
    assert!(body["remainingDays"].is_null());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_remaining_days_positive_for_active_period() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let today = chrono::Utc::now().date_naive();
    let start = today - chrono::Duration::days(5);
    let end = today + chrono::Duration::days(25);
    let start_str = start.format("%Y-%m-%d").to_string();
    let end_str = end.format("%Y-%m-%d").to_string();

    let period_id = common::entities::create_period(&client, &start_str, &end_str).await;

    let resp = client.get(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["status"], "active");
    let remaining = body["remainingDays"].as_i64().unwrap();
    assert_eq!(remaining, 25, "remainingDays should be 25 for a period ending 25 days from now");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_discriminator_correct_on_duration_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "periodType": "Duration",
        "startDate": "2027-01-01",
        "name": "Duration Discriminator Test",
        "duration": {
            "durationUnits": 14,
            "durationUnit": "days"
        }
    });

    let create_resp = client
        .post(format!("{}/periods", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(create_resp.status(), Status::Created);
    let created: Value = serde_json::from_str(&create_resp.into_string().await.unwrap()).unwrap();

    // On create response, discriminator should be "duration"
    assert_eq!(created["periodType"], "duration");
    assert_eq!(created["duration"]["durationUnits"], 14);
    assert_eq!(created["duration"]["durationUnit"], "days");
    assert_eq!(created["length"], 14);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_number_of_transactions_reflects_real_data() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create period spanning a date range
    let today = chrono::Utc::now().date_naive();
    let start = today - chrono::Duration::days(10);
    let end = today + chrono::Duration::days(10);
    let start_str = start.format("%Y-%m-%d").to_string();
    let end_str = end.format("%Y-%m-%d").to_string();

    let period_id = common::entities::create_period(&client, &start_str, &end_str).await;

    // Create account and category for transactions
    let account_id = common::entities::create_account(&client, "Txn Test Account", 100_000).await;
    let category_id = common::entities::create_category(&client, "Txn Test Category", "expense").await;

    // Create 2 transactions within the period date range
    let tx_date = today.format("%Y-%m-%d").to_string();
    common::entities::create_transaction(&client, &account_id, &category_id, 5_000, &tx_date).await;
    common::entities::create_transaction(&client, &account_id, &category_id, 3_000, &tx_date).await;

    // Create 1 transaction outside the period (noise)
    let outside_date = (start - chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    common::entities::create_transaction(&client, &account_id, &category_id, 9_999, &outside_date).await;

    // GET the period and verify transaction count
    let resp = client.get(format!("{}/periods/{}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["numberOfTransactions"], 2, "numberOfTransactions should be 2 (not 3 - noise excluded)");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_periods_empty_state() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/periods", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["totalCount"], 0);
    assert_eq!(body["hasMore"], false);
}
