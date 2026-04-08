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

/// Creates a unique user via V2 register, sets currency (EUR), and returns `(user_id, email)`.
/// The client retains the session cookie set by register.
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
        "currency": "EUR"
    });
    let profile_resp = client
        .put(format!("{}/settings/profile", super::V2_BASE))
        .header(ContentType::JSON)
        .body(profile_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(profile_resp.status(), Status::Ok, "set currency on profile failed");

    (user_id, email)
}
