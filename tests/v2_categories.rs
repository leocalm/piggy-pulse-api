mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /categories (create)
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
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
    assert_eq!(body["status"], "active");
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_income() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "name": "Salary",
        "type": "income",
        "icon": "💰",
        "color": "#00ff00",
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
    assert_eq!(body["type"], "income");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_create_category_name_too_short() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
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
async fn test_create_category_bad_color() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "name": "Bad Color",
        "type": "expense",
        "icon": "🛒",
        "color": "not-hex",
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
async fn test_create_category_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
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
async fn test_list_categories_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_category(&client, "Cat One", "expense").await;

    let resp = client.get(format!("{}/categories", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_categories_pagination() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    for i in 0..3 {
        common::entities::create_category(&client, &format!("PageCat {}", i), "expense").await;
    }

    let resp = client.get(format!("{}/categories?limit=1", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    common::assertions::assert_paginated(&body);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["hasMore"], true);
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
async fn test_update_category_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Old Name", "expense").await;

    let payload = serde_json::json!({
        "name": "New Name",
        "type": "expense",
        "icon": "🍕",
        "color": "#ff0000",
        "description": "Updated",
        "parentId": null
    });

    let resp = client
        .put(format!("{}/categories/{}", V2_BASE, cat_id))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "New Name");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_category_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({
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
        .body("{}")
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /categories/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_category_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "To Delete", "expense").await;

    let resp = client.delete(format!("{}/categories/{}", V2_BASE, cat_id)).dispatch().await;

    assert_eq!(resp.status(), Status::NoContent);
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
async fn test_archive_category_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "To Archive", "expense").await;

    let resp = client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
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
async fn test_unarchive_category_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let cat_id = common::entities::create_category(&client, "Unarchive Me", "expense").await;

    client.post(format!("{}/categories/{}/archive", V2_BASE, cat_id)).dispatch().await;

    let resp = client.post(format!("{}/categories/{}/unarchive", V2_BASE, cat_id)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_unarchive_category_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/categories/{}/unarchive", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /categories/options & /categories/overview
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_options_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    common::entities::create_category(&client, "Option Cat", "expense").await;

    let resp = client.get(format!("{}/categories/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body.is_array());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_options_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/categories/options", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_overview_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/categories/overview", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_category_overview_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/categories/overview", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
