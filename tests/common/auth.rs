use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;
use uuid::Uuid;

/// Fetches the EUR currency ID via the V2 currency endpoint.
/// Does NOT require authentication.
pub async fn get_eur_currency_id(client: &Client) -> String {
    let resp = client.get(format!("{}/currencies/EUR", super::V2_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("currency body")).expect("valid json");
    body["id"].as_str().expect("currency id").to_string()
}

/// Fetches the EUR currency ID without requiring an existing session.
/// The V2 currencies endpoint is public, so this is identical to `get_eur_currency_id`.
pub async fn get_eur_currency_id_unauthenticated(client: &Client) -> String {
    get_eur_currency_id(client).await
}

/// Unlocks the session with a test DEK (all zeros for integration tests).
/// This must be called after login/register for any authenticated operations.
pub async fn unlock_session(client: &Client) {
    // Generate a test DEK (32 bytes of zeros, base64-encoded)
    let test_dek = BASE64.encode([0u8; 32]);

    let payload = serde_json::json!({
        "dek": test_dek
    });

    let resp = client
        .post(format!("{}/auth/unlock", super::V2_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(resp.status(), Status::NoContent, "unlock failed: {:?}", resp.status());
}

/// Creates a unique user via V2 register, sets currency (EUR), and returns `(user_id, email)`.
/// The client retains the session cookie set by register.
/// Also unlocks the session for encrypted operations.
pub async fn create_user_and_login(client: &Client) -> (String, String) {
    let unique = Uuid::new_v4();
    let email = format!("test.{}@example.com", unique);

    let register_payload = serde_json::json!({
        "name": format!("Test User {}", unique),
        "email": email,
        "password": super::TEST_PASSWORD,
    });

    let resp = client
        .post(format!("{}/auth/register", super::V2_BASE))
        .header(ContentType::JSON)
        .body(register_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("register body")).expect("valid json");
    let user_id = body["user"]["id"].as_str().expect("user id").to_string();
    let name = body["user"]["name"].as_str().unwrap_or("Test User").to_string();

    // Set default currency (required as first onboarding step before accounts can be created)
    let profile_payload = serde_json::json!({
        "name": name,
        "currency": "EUR",
        "avatar": "🐷"
    });
    let profile_resp = client
        .put(format!("{}/settings/profile", super::V2_BASE))
        .header(ContentType::JSON)
        .body(profile_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(profile_resp.status(), Status::Ok, "set currency on profile failed");

    // Unlock session for encrypted operations
    unlock_session(client).await;

    // Reset structure to create the system Transfer category and clean slate
    let reset_payload = serde_json::json!({
        "password": super::TEST_PASSWORD
    });
    let reset_resp = client
        .post(format!("{}/settings/reset-structure", super::V2_BASE))
        .header(ContentType::JSON)
        .body(reset_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(reset_resp.status(), Status::NoContent, "reset-structure failed");

    (user_id, email)
}
