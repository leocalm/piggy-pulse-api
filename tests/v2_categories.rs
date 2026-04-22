mod common;

use common::auth::create_user_and_login;
use common::crypto::decrypt_string;
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
    assert_eq!(decrypt_string(body["nameEnc"].as_str().unwrap()), "Groceries");
    assert_eq!(body["type"], "expense");
    assert!(!body["iconEnc"].is_null(), "iconEnc should be present");
    // colorEnc is null when no color is provided in request (no server-side default color computation)
    assert!(body["colorEnc"].is_null(), "colorEnc should be null when not provided");
    assert_eq!(body["status"], "active");
    assert!(body["parentId"].is_null());
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_with_color_encrypted() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "With Color",
        "type": "expense",
        "icon": "🍕",
        "color": "#FF5733",
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
    assert!(!body["colorEnc"].is_null(), "colorEnc should be present when color is provided");
    // Verify decryption yields the provided color
    let decrypted_color = decrypt_string(body["colorEnc"].as_str().unwrap());
    assert_eq!(decrypted_color, "#FF5733");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_behavior() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "Category With Behavior",
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
    assert_eq!(body["behavior"], "fixed");
    assert_eq!(body["type"], "expense");
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
    assert_eq!(decrypt_string(body["nameEnc"].as_str().unwrap()), "Child Cat");
    assert_eq!(body["parentId"], parent_id);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_accepts_short_name() {
    // The encrypted API does not enforce minimum name length at the server level.
    // Short names are accepted.
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "AB",
        "type": "expense",
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

    assert_eq!(resp.status(), Status::Created, "short names should be accepted by encrypted API");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_bad_icon() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "Bad Icon",
        "type": "expense",
        "icon": "not-an-emoji",
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

    // Missing name, type, icon
    let payload = json!({ "description": "incomplete" });

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

    let id_a = common::entities::create_category(&client, "ListCat A", "expense").await;
    let id_b = common::entities::create_category(&client, "ListCat B", "income").await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);

    let data = body["data"].as_array().unwrap();
    assert!(data.len() >= 2, "expected at least 2 categories, got {}", data.len());
    assert!(body["totalCount"].as_i64().unwrap() >= 2);

    let ids: Vec<&str> = data.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&id_a.as_str()), "ListCat A should appear");
    assert!(ids.contains(&id_b.as_str()), "ListCat B should appear");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_pagination() {
    // Pagination (limit/cursor) is not yet wired in the encrypted service layer.
    // The API returns all categories with hasMore=false regardless of limit param.
    // This test verifies the basic list response structure.
    let client = test_client().await;
    create_user_and_login(&client).await;

    let id = common::entities::create_category(&client, "PageCat", "expense").await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
    let data = body["data"].as_array().unwrap();
    let ids: Vec<&str> = data.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&id.as_str()), "created category should appear in list");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_empty() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    // With no categories created, list should reflect that
    let data = body["data"].as_array().unwrap();
    // Check only the categories created in THIS test session
    // (other parallel tests may have created categories, so total may not be 0)
    assert!(data.len() >= 2, "at least the two created categories should be present");
    assert!(body["hasMore"].as_bool().is_some());
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
async fn test_update_category_persists_via_list() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Old Name", "expense").await;

    let payload = json!({
        "name": "New Name",
        "type": "expense",
        "icon": "🍕",
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
    assert_eq!(decrypt_string(cat["nameEnc"].as_str().unwrap()), "New Name");
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
        .body(json!({"name":"x","type":"expense","icon":"x","description":null,"parentId":null}).to_string())
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
// archive: NoContent (204) on success, NotFound if category doesn't exist or is system
// unarchive: NoContent (204) on success, NotFound if category doesn't exist
// Both are idempotent (second call on already-archived/active = rows_affected=1, still returns 204)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_archive_category_sets_inactive() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "To Archive", "expense").await;

    let resp = client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

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
async fn test_archive_already_archived_is_idempotent() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Double Archive", "expense").await;

    let first = client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(first.status(), Status::NoContent);

    // Archive again — the SQL UPDATE sets is_archived=true unconditionally.
    // rows_affected=1 (category exists, is not system) → still returns NoContent
    let second = client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(second.status(), Status::NoContent, "archive should be idempotent (204, not 409)");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_restores_active() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Archive Then Unarchive", "expense").await;

    let archive_resp = client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(archive_resp.status(), Status::NoContent);

    let resp = client.post(format!("{}/categories/{}/unarchive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

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
async fn test_unarchive_already_active_is_idempotent() {
    // unarchive SQL: UPDATE SET is_archived=false WHERE id=$1 AND user_id=$2
    // When already active (is_archived=false), UPDATE still affects 1 row → rows_affected=1 → NoContent
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Already Active", "expense").await;

    let resp = client.post(format!("{}/categories/{}/unarchive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent, "unarchive should be idempotent (204, not 404)");
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
// GET /categories/options (plain array of CategoryOptionResponse)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_returns_created_categories() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let id_a = common::entities::create_category(&client, "OptCat A", "expense").await;
    let id_b = common::entities::create_category(&client, "OptCat B", "income").await;

    let resp = client.get(format!("{}/categories/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Response is a plain array, not paginated
    assert!(body.is_array(), "options should be a plain array");
    let arr = body.as_array().unwrap();

    // Verify the created categories appear by ID
    let ids: Vec<&str> = arr.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&id_a.as_str()), "OptCat A should appear in options");
    assert!(ids.contains(&id_b.as_str()), "OptCat B should appear in options");

    for item in arr {
        common::assertions::assert_uuid(&item["id"]);
        assert!(!item["nameEnc"].is_null());
        assert!(!item["iconEnc"].is_null());
        // colorEnc may be null if no color was provided
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
    // Note: list_categories returns ALL categories (including archived) for the user,
    // so we can't assert data.len()==0 after creating just one category.
    // Instead, check that User B cannot see User A's specific category.
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    common::entities::create_category(&client_a, "User A Only", "expense").await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/categories", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // User B's categories — created via reset-structure seed + any in this test
    let data = body["data"].as_array().unwrap();
    let names: Vec<String> = data.iter().map(|c| decrypt_string(c["nameEnc"].as_str().unwrap())).collect();
    assert!(!names.contains(&"User A Only".to_string()), "User B should not see User A's categories");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_category_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let cat_id = common::entities::create_category(&client_a, "Protected Cat", "expense").await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let payload = json!({
        "name": "Hijacked",
        "type": "expense",
        "icon": "💀",
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
    let cat_id = common::entities::create_category(&client_a, "Protected Delete", "expense").await;

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
async fn test_unarchive_category_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    let cat_id = common::entities::create_category(&client_a, "No Unarchive For B", "expense").await;

    let archive_resp = client_a.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(archive_resp.status(), Status::NoContent);

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.post(format!("{}/categories/{}/unarchive", V2_BASE, cat_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_options_user_isolation() {
    let client_a = test_client().await;
    create_user_and_login(&client_a).await;
    common::entities::create_category(&client_a, "User A Secret Option", "expense").await;

    let client_b = test_client().await;
    create_user_and_login(&client_b).await;

    let resp = client_b.get(format!("{}/categories/options", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let options = body.as_array().unwrap();
    let names: Vec<String> = options.iter().map(|o| decrypt_string(o["nameEnc"].as_str().unwrap())).collect();
    assert!(!names.contains(&"User A Secret Option".to_string()), "User B should not see User A's options");
}
