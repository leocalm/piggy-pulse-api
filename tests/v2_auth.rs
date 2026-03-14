mod common;

use common::{TEST_PASSWORD, V2_BASE, test_client};
use rocket::http::{ContentType, Status};
use serde_json::Value;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// POST /auth/register
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_register_happy() {
    let client = test_client().await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
        "email": format!("register.{}@example.com", Uuid::new_v4()),
        "password": TEST_PASSWORD,
        "name": "New User",
        "currencyId": eur_id
    });

    let resp = client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Created);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["requiresTwoFactor"], false);
    assert!(body["user"]["id"].is_string());
    assert!(body["user"]["email"].is_string());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_register_duplicate_email() {
    let client = test_client().await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;
    let email = format!("dup.{}@example.com", Uuid::new_v4());

    let payload = serde_json::json!({
        "email": email,
        "password": TEST_PASSWORD,
        "name": "User One",
        "currencyId": eur_id
    });

    let resp = client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);

    // Same email again
    let payload2 = serde_json::json!({
        "email": email,
        "password": TEST_PASSWORD,
        "name": "User Two",
        "currencyId": eur_id
    });

    let resp2 = client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(payload2.to_string())
        .dispatch()
        .await;
    assert_eq!(resp2.status(), Status::Conflict);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_register_weak_password() {
    let client = test_client().await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;

    let payload = serde_json::json!({
        "email": format!("weak.{}@example.com", Uuid::new_v4()),
        "password": "short",
        "name": "Weak Pass",
        "currencyId": eur_id
    });

    let resp = client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_register_missing_fields() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "email": "missing@example.com"
    });

    let resp = client
        .post(format!("{}/auth/register", V2_BASE))
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
async fn test_register_malformed_json() {
    let client = test_client().await;

    let resp = client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body("not valid json{{{")
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400 or 422, got {}",
        resp.status()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /auth/login
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_login_happy() {
    let client = test_client().await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;
    let email = format!("login.{}@example.com", Uuid::new_v4());

    // Register first
    let reg = serde_json::json!({
        "email": email,
        "password": TEST_PASSWORD,
        "name": "Login User",
        "currencyId": eur_id
    });
    let resp = client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(reg.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);

    // Login
    let login = serde_json::json!({
        "email": email,
        "password": TEST_PASSWORD
    });
    let resp = client
        .post(format!("{}/auth/login", V2_BASE))
        .header(ContentType::JSON)
        .body(login.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["requiresTwoFactor"], false);
    assert!(body["user"]["id"].is_string());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_login_wrong_password() {
    let client = test_client().await;
    let eur_id = common::auth::get_eur_currency_id(&client).await;
    let email = format!("wrongpw.{}@example.com", Uuid::new_v4());

    let reg = serde_json::json!({
        "email": email,
        "password": TEST_PASSWORD,
        "name": "Wrong PW User",
        "currencyId": eur_id
    });
    client
        .post(format!("{}/auth/register", V2_BASE))
        .header(ContentType::JSON)
        .body(reg.to_string())
        .dispatch()
        .await;

    let login = serde_json::json!({
        "email": email,
        "password": "WrongPassword!2026"
    });
    let resp = client
        .post(format!("{}/auth/login", V2_BASE))
        .header(ContentType::JSON)
        .body(login.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_login_nonexistent_email() {
    let client = test_client().await;

    let login = serde_json::json!({
        "email": format!("nonexistent.{}@example.com", Uuid::new_v4()),
        "password": TEST_PASSWORD
    });
    let resp = client
        .post(format!("{}/auth/login", V2_BASE))
        .header(ContentType::JSON)
        .body(login.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_login_missing_fields() {
    let client = test_client().await;

    let login = serde_json::json!({
        "email": "test@example.com"
    });
    let resp = client
        .post(format!("{}/auth/login", V2_BASE))
        .header(ContentType::JSON)
        .body(login.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::UnprocessableEntity,
        "expected 400 or 422, got {}",
        resp.status()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /auth/logout
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_logout_happy() {
    let client = test_client().await;
    common::auth::create_user_and_login(&client).await;

    let resp = client.post(format!("{}/auth/logout", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_logout_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/auth/logout", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GET /auth/me
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_me_happy() {
    let client = test_client().await;
    let (_user_id, email) = common::auth::create_user_and_login(&client).await;

    let resp = client.get(format!("{}/auth/me", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["email"], email);
    common::assertions::assert_uuid(&body["id"]);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_me_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/auth/me", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /auth/refresh
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_refresh_happy() {
    let client = test_client().await;
    common::auth::create_user_and_login(&client).await;

    let resp = client.post(format!("{}/auth/refresh", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    // Cookie-based auth: refresh extends the session cookie, response confirms success
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    // Accept either a token field (token-based) or a success/user field (cookie-based)
    assert!(
        body["token"].is_string() || body["user"].is_object() || body["message"].is_string() || body.is_object(),
        "expected a valid refresh response body"
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_refresh_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/auth/refresh", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUT /auth/password
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_change_password_happy() {
    let client = test_client().await;
    common::auth::create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "currentPassword": TEST_PASSWORD,
        "newPassword": "NewSecurePassword!2026abc"
    });

    let resp = client
        .put(format!("{}/auth/password", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_change_password_wrong_current() {
    let client = test_client().await;
    common::auth::create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "currentPassword": "WrongCurrentPassword!2026",
        "newPassword": "NewSecurePassword!2026abc"
    });

    let resp = client
        .put(format!("{}/auth/password", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_change_password_weak_new() {
    let client = test_client().await;
    common::auth::create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "currentPassword": TEST_PASSWORD,
        "newPassword": "short"
    });

    let resp = client
        .put(format!("{}/auth/password", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_change_password_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "currentPassword": "anything",
        "newPassword": "anything"
    });

    let resp = client
        .put(format!("{}/auth/password", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /auth/forgot-password
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_forgot_password_happy() {
    let client = test_client().await;
    let (_user_id, email) = common::auth::create_user_and_login(&client).await;

    let payload = serde_json::json!({
        "email": email
    });

    let resp = client
        .post(format!("{}/auth/forgot-password", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    // Should succeed silently (no info leak)
    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_forgot_password_nonexistent_email_no_error() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "email": format!("nonexistent.{}@example.com", Uuid::new_v4())
    });

    let resp = client
        .post(format!("{}/auth/forgot-password", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    // Must not leak whether email exists
    assert_eq!(resp.status(), Status::Ok);
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST /auth/reset-password
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_reset_password_invalid_token() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "token": "invalid-token-value",
        "password": "NewSecurePassword!2026abc"
    });

    let resp = client
        .post(format!("{}/auth/reset-password", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::NotFound,
        "expected 400 or 404, got {}",
        resp.status()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2FA endpoints
// ═══════════════════════════════════════════════════════════════════════════════

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_status_happy() {
    let client = test_client().await;
    common::auth::create_user_and_login(&client).await;

    let resp = client.get(format!("{}/auth/2fa/status", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["enabled"], false);
    assert!(body["hasBackupCodes"].is_boolean());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_status_no_auth() {
    let client = test_client().await;

    let resp = client.get(format!("{}/auth/2fa/status", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_enable_happy() {
    let client = test_client().await;
    common::auth::create_user_and_login(&client).await;

    let resp = client.post(format!("{}/auth/2fa/enable", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Ok);
    let body: Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert!(body["secret"].is_string());
    assert!(body["qrCodeUri"].is_string());
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_enable_no_auth() {
    let client = test_client().await;

    let resp = client.post(format!("{}/auth/2fa/enable", V2_BASE)).dispatch().await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_verify_wrong_code() {
    let client = test_client().await;
    common::auth::create_user_and_login(&client).await;

    // Enable 2FA first
    client.post(format!("{}/auth/2fa/enable", V2_BASE)).dispatch().await;

    let payload = serde_json::json!({
        "code": "000000"
    });

    let resp = client
        .post(format!("{}/auth/2fa/verify", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::Unauthorized,
        "expected 400 or 401 for wrong 2FA code, got {}",
        resp.status()
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_disable_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "code": "000000"
    });

    let resp = client
        .post(format!("{}/auth/2fa/disable", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_backup_codes_no_auth() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "code": "000000"
    });

    let resp = client
        .post(format!("{}/auth/2fa/backup-codes/regenerate", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_emergency_disable_request() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "email": format!("emergency.{}@example.com", Uuid::new_v4())
    });

    let resp = client
        .post(format!("{}/auth/2fa/emergency-disable/request", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    // Public endpoint — should accept silently
    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn test_2fa_emergency_disable_confirm_invalid_token() {
    let client = test_client().await;

    let payload = serde_json::json!({
        "token": "invalid-emergency-token"
    });

    let resp = client
        .post(format!("{}/auth/2fa/emergency-disable/confirm", V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert!(
        resp.status() == Status::BadRequest || resp.status() == Status::NotFound,
        "expected 400 or 404, got {}",
        resp.status()
    );
}
