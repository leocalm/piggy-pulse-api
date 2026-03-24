mod common;

use common::auth::create_user_and_login;
use common::{V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /onboarding/category-templates
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_category_templates_happy() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/onboarding/category-templates", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let templates = body.as_array().expect("response is array");
    assert_eq!(templates.len(), 2, "should have exactly 2 templates");

    // Verify essential template
    let essential = templates.iter().find(|t| t["id"] == "essential").expect("essential template");
    assert_eq!(essential["name"], "Essential 5");
    let cats = essential["categories"].as_array().unwrap();
    assert_eq!(cats.len(), 5);

    // Verify detailed template
    let detailed = templates.iter().find(|t| t["id"] == "detailed").expect("detailed template");
    assert_eq!(detailed["name"], "Detailed 12");
    let cats = detailed["categories"].as_array().unwrap();
    assert_eq!(cats.len(), 12);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_category_templates_each_has_required_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/onboarding/category-templates", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    for template in body.as_array().unwrap() {
        assert!(template["id"].is_string(), "template has id");
        assert!(template["name"].is_string(), "template has name");
        assert!(template["description"].is_string(), "template has description");
        assert!(template["categories"].is_array(), "template has categories");

        for cat in template["categories"].as_array().unwrap() {
            assert!(cat["name"].is_string(), "category has name");
            assert!(cat["type"].is_string(), "category has type");
            assert!(cat["icon"].is_string(), "category has icon");
        }
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_category_templates_unauthenticated_returns_401() {
    let client = test_client().await;

    let resp = client.get(format!("{}/onboarding/category-templates", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /onboarding/apply-template
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_apply_essential_template() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({ "templateId": "essential" });

    let resp = client
        .post(format!("{}/onboarding/apply-template", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    let categories = body.as_array().expect("response is array");
    assert_eq!(categories.len(), 5);

    // Each returned category should have the standard fields
    for cat in categories {
        assert!(cat["id"].is_string());
        assert!(cat["name"].is_string());
        assert!(cat["type"].is_string());
        assert!(cat["icon"].is_string());
        assert!(cat["color"].is_string());
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_apply_detailed_template() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({ "templateId": "detailed" });

    let resp = client
        .post(format!("{}/onboarding/apply-template", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 12);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_apply_unknown_template_returns_404() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = serde_json::json!({ "templateId": "nonexistent" });

    let resp = client
        .post(format!("{}/onboarding/apply-template", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_apply_template_unauthenticated_returns_401() {
    let client = test_client().await;

    let payload = serde_json::json!({ "templateId": "essential" });

    let resp = client
        .post(format!("{}/onboarding/apply-template", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_apply_template_creates_categories_visible_in_list() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Apply essential template
    let payload = serde_json::json!({ "templateId": "essential" });
    let apply_resp = client
        .post(format!("{}/onboarding/apply-template", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(apply_resp.status(), Status::Created);

    // List categories — essential template creates 5 (plus the system Transfer category)
    let list_resp = client.get(format!("{}/categories?limit=50", V2_BASE)).dispatch().await;
    assert_eq!(list_resp.status(), Status::Ok);
    let list_body: Value = serde_json::from_str(&list_resp.into_string().await.unwrap()).unwrap();
    let total = list_body["totalCount"].as_i64().unwrap_or(0);
    // At minimum 5 categories were created (there may also be a system Transfer)
    assert!(total >= 5);
}
