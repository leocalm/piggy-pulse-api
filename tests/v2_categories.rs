mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /categories (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "Groceries",
        "type": "expense",
        "icon": "🛒",
        "color": "#00aa55",
        "description": "Weekly groceries",
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
    assert_eq!(body["name"], "Groceries");
    assert_eq!(body["type"], "expense");
    assert_eq!(body["icon"], "🛒");
    assert_eq!(body["color"], "#00aa55");
    assert_eq!(body["status"], "active");
    assert!(body["parentId"].is_null());
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_with_parent() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let parent_id = common::entities::create_category(&client, "Parent Cat", "expense").await;

    let payload = json!({
        "name": "Child Cat",
        "type": "expense",
        "icon": "🍎",
        "color": "#ff0000",
        "description": null,
        "parentId": parent_id
    });

    let resp = client
        .post(format!("{}/categories", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "Child Cat");
    assert_eq!(body["parentId"], parent_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_name_too_short() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "AB",
        "type": "expense",
        "icon": "🛒",
        "color": "#000000",
        "description": null,
        "parentId": null
    });

    let resp = client
        .post(format!("{}/categories", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_missing_required_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Missing name, type, icon, color
    let payload = json!({
        "description": "incomplete"
    });

    let resp = client
        .post(format!("{}/categories", V2_BASE))
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
async fn test_create_category_no_auth() {
    let client = test_client().await;

    let payload = json!({
        "name": "No Auth",
        "type": "expense",
        "icon": "🛒",
        "color": "#000000",
        "description": null,
        "parentId": null
    });

    let resp = client
        .post(format!("{}/categories", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /categories (list)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_returns_created() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    common::entities::create_category(&client, "ListCat A", "expense").await;
    common::entities::create_category(&client, "ListCat B", "income").await;
    common::entities::create_category(&client, "ListCat C", "expense").await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);

    let data = body["data"].as_array().unwrap();
    assert!(data.len() >= 3, "expected at least 3 categories, got {}", data.len());
    assert!(body["totalCount"].as_i64().unwrap() >= 3);

    let names: Vec<&str> = data.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"ListCat A"));
    assert!(names.contains(&"ListCat B"));
    assert!(names.contains(&"ListCat C"));
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_number_of_transactions_zero() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "ZeroTxCat", "expense").await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let data = body["data"].as_array().unwrap();
    let cat = data.iter().find(|c| c["id"].as_str().unwrap() == cat_id).unwrap();
    assert_eq!(cat["numberOfTransactions"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_number_of_transactions_reflects_created() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "TxCountCat", "expense").await;
    let account_id = common::entities::create_account(&client, "TxCountAcct", 100_000).await;
    common::entities::create_transaction(&client, &account_id, &cat_id, 5_000, "2026-03-10").await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let data = body["data"].as_array().unwrap();
    let cat = data.iter().find(|c| c["id"].as_str().unwrap() == cat_id).unwrap();
    assert_eq!(cat["numberOfTransactions"], 1, "expected 1 transaction on category");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_pagination() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    common::entities::create_category(&client, "PageCat 0", "expense").await;
    common::entities::create_category(&client, "PageCat 1", "expense").await;
    common::entities::create_category(&client, "PageCat 2", "expense").await;

    // First page
    let resp = client.get(format!("{}/categories?limit=1", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["hasMore"], true);

    // Cursor through to second page
    let cursor = body["nextCursor"].as_str().unwrap();
    let resp2 = client.get(format!("{}/categories?limit=1&cursor={}", V2_BASE, cursor)).dispatch().await;
    assert_eq!(resp2.status(), Status::Ok);
    let body2: Value = serde_json::from_str(&resp2.into_string().await.unwrap()).unwrap();
    assert_eq!(body2["data"].as_array().unwrap().len(), 1);
    // Second page item should differ from first
    assert_ne!(body["data"][0]["id"].as_str().unwrap(), body2["data"][0]["id"].as_str().unwrap());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_empty() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["totalCount"], 0);
    assert_eq!(body["hasMore"], false);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /categories/{id} (update)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_category_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Old Name", "expense").await;

    let payload = json!({
        "name": "New Name",
        "type": "expense",
        "icon": "🍕",
        "color": "#ff0000",
        "description": "Updated desc",
        "parentId": null
    });

    let resp = client
        .put(format!("{}/categories/{}", V2_BASE, cat_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let put_body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(put_body["name"], "New Name");
    assert_eq!(put_body["color"], "#ff0000");

    // Verify via GET list — persistence, not just echo
    let list_resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let cat = list_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"].as_str().unwrap() == cat_id)
        .expect("updated category should appear in list");
    assert_eq!(cat["name"], "New Name");
    assert_eq!(cat["color"], "#ff0000");
    assert_eq!(cat["icon"], "🍕");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_category_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "Ghost",
        "type": "expense",
        "icon": "🛒",
        "color": "#000000",
        "description": null,
        "parentId": null
    });

    let resp = client
        .put(format!("{}/categories/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_category_no_auth() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/categories/{}", V2_BASE, Uuid::new_v4()))
        .header(ContentType::JSON)
        .body(json!({"name":"x","type":"expense","icon":"x","color":"#000","description":null,"parentId":null}).to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /categories/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_category_verified_via_list() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "To Delete", "expense").await;

    let resp = client.delete(format!("{}/categories/{}", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // Verify it no longer appears in list
    let list_resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let ids: Vec<&str> = list_body["data"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert!(!ids.contains(&cat_id.as_str()), "deleted category should not appear in list");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_category_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/categories/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_category_no_auth() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/categories/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /categories/{id}/archive & unarchive
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_category_sets_inactive() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "To Archive", "expense").await;

    let resp = client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "inactive");

    // Verify via GET list
    let list_resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let cat = list_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"].as_str().unwrap() == cat_id)
        .expect("archived category should still appear in list");
    assert_eq!(cat["status"], "inactive");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_already_archived_conflict() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Double Archive", "expense").await;

    let first = client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(first.status(), Status::Ok);

    let second = client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(second.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_restores_active() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Archive Then Unarchive", "expense").await;

    client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;

    let resp = client.post(format!("{}/categories/{}/unarchive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["status"], "active");

    // Verify via GET list
    let list_resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let cat = list_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"].as_str().unwrap() == cat_id)
        .unwrap();
    assert_eq!(cat["status"], "active");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_already_active_conflict() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Already Active", "expense").await;

    // Category is already active — unarchiving should conflict
    let resp = client.post(format!("{}/categories/{}/unarchive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_category_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.post(format!("{}/categories/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_category_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/categories/{}/archive", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_category_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/categories/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /categories/overview
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overview_total_spent_reflects_transactions() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_a = common::entities::create_category(&client, "OverviewCatA", "expense").await;
    let cat_b = common::entities::create_category(&client, "OverviewCatB", "expense").await;
    let account_id = common::entities::create_account(&client, "OverviewAcct", 500_000).await;
    let period_id = common::entities::create_period(&client, "2026-04-01", "2026-04-30").await;

    common::entities::create_transaction(&client, &account_id, &cat_a, 3_000, "2026-04-05").await;
    common::entities::create_transaction(&client, &account_id, &cat_b, 7_000, "2026-04-10").await;

    let resp = client.get(format!("{}/categories/overview?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    assert_eq!(body["summary"]["totalSpent"], 10_000, "totalSpent should be 3000 + 7000");

    let categories = body["categories"].as_array().unwrap();
    let overview_a = categories.iter().find(|c| c["id"].as_str().unwrap() == cat_a).unwrap();
    let overview_b = categories.iter().find(|c| c["id"].as_str().unwrap() == cat_b).unwrap();
    assert_eq!(overview_a["actual"], 3_000);
    assert_eq!(overview_b["actual"], 7_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overview_category_with_target_shows_budgeted() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat_id = common::entities::create_category(&client, "BudgetedCat", "expense").await;
    let account_id = common::entities::create_account(&client, "BudgetAcct", 500_000).await;
    let period_id = common::entities::create_period(&client, "2026-05-01", "2026-05-31").await;

    common::entities::create_target(&client, &cat_id, 40_000).await;
    common::entities::create_transaction(&client, &account_id, &cat_id, 15_000, "2026-05-10").await;

    let resp = client.get(format!("{}/categories/overview?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let categories = body["categories"].as_array().unwrap();
    let cat = categories.iter().find(|c| c["id"].as_str().unwrap() == cat_id).unwrap();
    assert_eq!(cat["actual"], 15_000);
    assert_eq!(cat["budgeted"], 40_000);
    // variance = budgeted - actual
    assert_eq!(cat["variance"], 40_000 - 15_000);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overview_category_without_target_budgeted_null() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    common::entities::create_category(&client, "NoBudgetCat", "expense").await;
    let period_id = common::entities::create_period(&client, "2026-06-01", "2026-06-30").await;

    let resp = client.get(format!("{}/categories/overview?periodId={}", V2_BASE, period_id)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let categories = body["categories"].as_array().unwrap();
    assert!(!categories.is_empty(), "should have at least the created category");
    let cat = categories.iter().find(|c| c["name"].as_str().unwrap() == "NoBudgetCat").unwrap();
    assert!(cat["budgeted"].is_null(), "budgeted should be null without target");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overview_missing_period_id() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/categories/overview", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_overview_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/categories/overview", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /categories/options
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_returns_created_categories() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    common::entities::create_category(&client, "OptCat A", "expense").await;
    common::entities::create_category(&client, "OptCat B", "income").await;

    let resp = client.get(format!("{}/categories/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Response is a plain array, not paginated
    assert!(body.is_array(), "options should be a plain array");
    let arr = body.as_array().unwrap();
    assert!(arr.len() >= 2);

    let names: Vec<&str> = arr.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"OptCat A"));
    assert!(names.contains(&"OptCat B"));

    // Each item should have id, name, icon, color
    for item in arr {
        common::assertions::assert_uuid(&item["id"]);
        assert!(item["name"].is_string());
        assert!(item["icon"].is_string());
        assert!(item["color"].is_string());
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/categories/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// User isolation
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    common::entities::create_category(&client_a, "User A Cat", "expense").await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["totalCount"], 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_category_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let cat_id = common::entities::create_category(&client_a, "User A Only", "expense").await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let payload = json!({
        "name": "Hijacked",
        "type": "expense",
        "icon": "💀",
        "color": "#ff0000",
        "description": null,
        "parentId": null
    });

    let resp = client_b
        .put(format!("{}/categories/{}", V2_BASE, cat_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_category_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let cat_id = common::entities::create_category(&client_a, "Protected Cat", "expense").await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.delete(format!("{}/categories/{}", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_category_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let cat_id = common::entities::create_category(&client_a, "No Archive For B", "expense").await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    common::entities::create_category(&client_a, "User A Option", "expense").await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/categories/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body.as_array().unwrap().is_empty(), "User B should see no options from User A");
}
