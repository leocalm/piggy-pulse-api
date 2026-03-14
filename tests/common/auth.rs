use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;
use uuid::Uuid;

/// Fetches the EUR currency ID via the V1 currency endpoint.
/// Requires an authenticated session (call after login/register).
pub async fn get_eur_currency_id(client: &Client) -> String {
    let resp = client.get(format!("{}/currency/EUR", super::V1_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("currency body")).expect("valid json");
    body["id"].as_str().expect("currency id").to_string()
}

/// Fetches the EUR currency ID by first creating a throwaway V1 user to get auth.
/// Use this when no session exists yet.
pub async fn get_eur_currency_id_unauthenticated(client: &Client) -> String {
    // Create a throwaway user via V1 to establish a session
    let throwaway_email = format!("throwaway.{}@example.com", Uuid::new_v4());
    let payload = serde_json::json!({
        "name": "Throwaway",
        "email": throwaway_email,
        "password": super::TEST_PASSWORD
    });
    let resp = client
        .post(format!("{}/users/", super::V1_BASE))
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);

    // Now we have a session cookie, fetch currency
    let eur_id = get_eur_currency_id(client).await;

    // Log out the throwaway user
    let _ = client.post(format!("{}/users/logout", super::V1_BASE)).dispatch().await;

    eur_id
}

/// Creates a unique user via V2 register and returns `(user_id, email)`.
/// The client retains the session cookie set by register.
pub async fn create_user_and_login(client: &Client) -> (String, String) {
    let unique = Uuid::new_v4();
    let email = format!("test.{}@example.com", unique);

    // Get EUR currency ID (needs a throwaway auth session)
    let eur_id = get_eur_currency_id_unauthenticated(client).await;

    let register_payload = serde_json::json!({
        "name": format!("Test User {}", unique),
        "email": email,
        "password": super::TEST_PASSWORD,
        "currencyId": eur_id
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

    (user_id, email)
}
