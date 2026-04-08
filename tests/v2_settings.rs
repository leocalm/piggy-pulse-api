mod common;

use common::auth::create_user_and_login;
use common::entities::{create_account, create_category, create_transaction};
use common::{TEST_PASSWORD, V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// GET /settings/profile
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_profile_matches_registration() {
    let client = test_client().await;
    let (_user_id, _email) = create_user_and_login(&client).await;

    let resp = client.get(format!("{}/settings/profile", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Name was set during registration as "Test User <uuid>" — assert it starts with "Test User"
    let name = body["name"].as_str().expect("name");
    assert!(name.starts_with("Test User"), "expected name to start with 'Test User', got '{}'", name);

    // Currency was set to EUR during registration
    assert_eq!(body["currency"], "EUR");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_profile_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "Updated Name",
        "currency": "USD",
        "avatar": "🐷"
    });

    let resp = client
        .put(format!("{}/settings/profile", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "Updated Name");
    assert_eq!(body["currency"], "USD");

    // Verify persistence via GET
    let resp = client.get(format!("{}/settings/profile", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["name"], "Updated Name");
    assert_eq!(body["currency"], "USD");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_profile_empty_name_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "",
        "currency": "EUR",
        "avatar": "🐷"
    });

    let resp = client
        .put(format!("{}/settings/profile", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_profile_invalid_currency_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "name": "Valid Name",
        "currency": "INVALID",
        "avatar": "🐷"
    });

    let resp = client
        .put(format!("{}/settings/profile", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_profile_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/settings/profile", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_profile_no_auth() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/settings/profile", V2_BASE))
        .header(ContentType::JSON)
        .body(json!({"name":"x","currency":"EUR","avatar":"🐷"}).to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /settings/preferences
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_preferences_has_all_fields() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/settings/preferences", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Assert all required fields exist and are the correct types
    assert!(body["theme"].is_string(), "expected theme string");
    assert!(body["dateFormat"].is_string(), "expected dateFormat string");
    assert!(body["numberFormat"].is_string(), "expected numberFormat string");
    assert!(body["language"].is_string(), "expected language string");
    assert!(body["compactMode"].is_boolean(), "expected compactMode boolean");
    assert!(body["dashboardLayout"].is_object(), "expected dashboardLayout object");

    // Defaults should be "light", "DD/MM/YYYY", "1,234.56", "en"
    assert_eq!(body["theme"], "light");
    assert_eq!(body["dateFormat"], "DD/MM/YYYY");
    assert_eq!(body["numberFormat"], "1,234.56");
    assert_eq!(body["language"], "en");

    // Default compactMode is false, dashboardLayout has empty arrays
    assert_eq!(body["compactMode"], false);
    assert_eq!(body["dashboardLayout"]["widgetOrder"].as_array().unwrap().len(), 0);
    assert_eq!(body["dashboardLayout"]["hiddenWidgets"].as_array().unwrap().len(), 0);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_get_preferences_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/settings/preferences", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /settings/preferences
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_preferences_persists_via_get() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "theme": "dark",
        "dateFormat": "YYYY-MM-DD",
        "numberFormat": "1.234,56",
        "language": "pt",
        "compactMode": true,
        "dashboardLayout": {
            "widgetOrder": ["overview", "accounts"],
            "hiddenWidgets": ["calendar"]
        }
    });

    let resp = client
        .put(format!("{}/settings/preferences", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["theme"], "dark");
    assert_eq!(body["dateFormat"], "YYYY-MM-DD");
    assert_eq!(body["numberFormat"], "1.234,56");
    assert_eq!(body["language"], "pt");
    assert_eq!(body["compactMode"], true);
    assert_eq!(body["dashboardLayout"]["widgetOrder"], json!(["overview", "accounts"]));
    assert_eq!(body["dashboardLayout"]["hiddenWidgets"], json!(["calendar"]));

    // Verify persistence via GET
    let resp = client.get(format!("{}/settings/preferences", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["theme"], "dark");
    assert_eq!(body["dateFormat"], "YYYY-MM-DD");
    assert_eq!(body["numberFormat"], "1.234,56");
    assert_eq!(body["language"], "pt");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_preferences_system_theme_persists() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "theme": "system",
        "dateFormat": "MM/DD/YYYY",
        "numberFormat": "1 234,56",
        "language": "en"
    });

    let resp = client
        .put(format!("{}/settings/preferences", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);

    // Verify via GET
    let resp = client.get(format!("{}/settings/preferences", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["theme"], "system");
    assert_eq!(body["dateFormat"], "MM/DD/YYYY");
    assert_eq!(body["numberFormat"], "1 234,56");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_preferences_invalid_theme_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "theme": "invalid_theme",
        "dateFormat": "DD/MM/YYYY",
        "numberFormat": "1,234.56",
        "language": "en"
    });

    let resp = client
        .put(format!("{}/settings/preferences", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    // Invalid theme value should be rejected (serde deserialization failure -> 422 or 400)
    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400 or 422, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_preferences_invalid_language_returns_400() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({
        "theme": "light",
        "dateFormat": "DD/MM/YYYY",
        "numberFormat": "1,234.56",
        "language": "123!invalid"
    });

    let resp = client
        .put(format!("{}/settings/preferences", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_update_preferences_no_auth() {
    let client = test_client().await;

    let resp = client
        .put(format!("{}/settings/preferences", V2_BASE))
        .header(ContentType::JSON)
        .body(json!({"theme":"light","dateFormat":"DD/MM/YYYY","numberFormat":"1,234.56","language":"en"}).to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /settings/sessions
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_sessions_has_current() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/settings/sessions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let sessions = body.as_array().expect("sessions array");

    assert!(!sessions.is_empty(), "should have at least current session");

    // At least one session must have isCurrent == true
    let has_current = sessions.iter().any(|s| s["isCurrent"].as_bool() == Some(true));
    assert!(has_current, "expected at least one session with isCurrent=true");

    // Every session must have id (UUID) and createdAt
    for s in sessions {
        common::assertions::assert_uuid(&s["id"]);
        assert!(s["createdAt"].is_string(), "expected createdAt string");
    }
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_list_sessions_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/settings/sessions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /settings/sessions/{id}
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_revoke_session_removes_from_list() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // List sessions, get the current session ID
    let resp = client.get(format!("{}/settings/sessions", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let sessions = body.as_array().unwrap();
    assert!(!sessions.is_empty());

    // Find any non-current session if available, or use current
    // Since we only have one login, there's only one session. Revoking it logs us out.
    // Instead, let's test the revoke-current flow here.
    let current = sessions.iter().find(|s| s["isCurrent"].as_bool() == Some(true)).unwrap();
    let session_id = current["id"].as_str().unwrap();

    // Revoke current session
    let resp = client.delete(format!("{}/settings/sessions/{}", V2_BASE, session_id)).dispatch().await;
    assert_eq!(resp.status(), Status::NoContent);

    // After revoking current session, subsequent requests should be unauthorized
    let resp = client.get(format!("{}/settings/sessions", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_revoke_session_not_found() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.delete(format!("{}/settings/sessions/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_revoke_session_no_auth() {
    let client = test_client().await;

    let resp = client.delete(format!("{}/settings/sessions/{}", V2_BASE, Uuid::new_v4())).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /settings/export/transactions
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_export_transactions_csv_with_real_amounts() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create test data
    let account_id = create_account(&client, "Export Account", 100_000).await;
    let category_id = create_category(&client, "Export Category", "expense").await;
    create_transaction(&client, &account_id, &category_id, 3000, "2026-03-01").await;
    create_transaction(&client, &account_id, &category_id, 5000, "2026-03-02").await;

    let resp = client.get(format!("{}/settings/export/transactions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let content_type = resp.content_type().expect("content type");
    assert!(content_type.to_string().contains("text/csv"), "expected text/csv, got {}", content_type);

    let body = resp.into_string().await.unwrap();

    // Assert CSV has headers
    assert!(body.starts_with("date,description,amount,currency,category,type,from_account,to_account,vendor\n"));

    // Assert both transaction amounts appear in the CSV
    assert!(body.contains("3000"), "expected amount 3000 in CSV, body: {}", body);
    assert!(body.contains("5000"), "expected amount 5000 in CSV, body: {}", body);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_export_transactions_empty_returns_headers_only() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let resp = client.get(format!("{}/settings/export/transactions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().await.unwrap();

    // Should have headers but no data rows
    assert_eq!(body.trim(), "date,description,amount,currency,category,type,from_account,to_account,vendor");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_export_transactions_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/settings/export/transactions", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /settings/export/data
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_export_data_contains_all_domains() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create entities across domains
    let account_id = create_account(&client, "Data Export Acct", 50_000).await;
    let category_id = create_category(&client, "Data Export Cat", "expense").await;
    create_transaction(&client, &account_id, &category_id, 1234, "2026-03-10").await;

    let resp = client.get(format!("{}/settings/export/data", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    // Assert required keys exist
    assert!(body["accounts"].is_array(), "expected accounts array");
    assert!(body["categories"].is_array(), "expected categories array");
    assert!(body["transactions"].is_array(), "expected transactions array");

    // Assert our data is actually present
    let accounts = body["accounts"].as_array().unwrap();
    assert!(accounts.iter().any(|a| a["name"].as_str() == Some("Data Export Acct")));

    let categories = body["categories"].as_array().unwrap();
    assert!(categories.iter().any(|c| c["name"].as_str() == Some("Data Export Cat")));

    let transactions = body["transactions"].as_array().unwrap();
    assert!(transactions.iter().any(|t| t["amount"].as_i64() == Some(1234)));
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_export_data_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/settings/export/data", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /settings/reset-structure
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_reset_structure_clears_data() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    // Create test data
    let account_id = create_account(&client, "Reset Account", 10_000).await;
    let category_id = create_category(&client, "Reset Category", "expense").await;
    create_transaction(&client, &account_id, &category_id, 500, "2026-03-01").await;

    // Verify data exists
    let resp = client.get(format!("{}/accounts", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(!body["data"].as_array().unwrap().is_empty(), "accounts should exist before reset");

    // Reset
    let payload = json!({ "password": TEST_PASSWORD });
    let resp = client
        .post(format!("{}/settings/reset-structure", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // After reset: accounts should be empty
    let resp = client.get(format!("{}/accounts", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 0, "accounts should be empty after reset");

    // After reset: export data should show no transactions
    let resp = client.get(format!("{}/settings/export/data", V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let txs = body["transactions"].as_array().unwrap();
    assert_eq!(txs.len(), 0, "transactions should be empty after reset");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_reset_structure_wrong_password() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({ "password": "WrongPassword!2026" });

    let resp = client
        .post(format!("{}/settings/reset-structure", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_reset_structure_no_auth() {
    let client = test_client().await;

    let resp = client
        .post(format!("{}/settings/reset-structure", V2_BASE))
        .header(ContentType::JSON)
        .body(json!({"password": "x"}).to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELETE /settings/account
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_account_then_login_fails() {
    let client = test_client().await;
    let (_user_id, email) = create_user_and_login(&client).await;

    // Delete account
    let payload = json!({ "password": TEST_PASSWORD });
    let resp = client
        .delete(format!("{}/settings/account", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    // Attempt to login with same credentials should fail
    let login_payload = json!({
        "email": email,
        "password": TEST_PASSWORD
    });
    let resp = client
        .post(format!("{}/auth/login", V2_BASE))
        .header(ContentType::JSON)
        .body(login_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_account_wrong_password() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let payload = json!({ "password": "WrongPassword!2026" });

    let resp = client
        .delete(format!("{}/settings/account", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_delete_account_no_auth() {
    let client = test_client().await;

    let resp = client
        .delete(format!("{}/settings/account", V2_BASE))
        .header(ContentType::JSON)
        .body(json!({"password":"x"}).to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}
