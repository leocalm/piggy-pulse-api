use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;
use uuid::Uuid;

/// Fetches the EUR currency ID via the V1 currency endpoint.
pub async fn get_eur_currency_id(client: &Client) -> String {
    let resp = client.get(format!("{}/currency/EUR", super::V1_BASE)).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("currency body")).expect("valid json");
    body["id"].as_str().expect("currency id").to_string()
}

/// Creates a unique user via V1 endpoints and logs in.
/// Returns `(user_id, email)`. The client retains the session cookie.
pub async fn create_user_and_login(client: &Client) -> (String, String) {
    let unique = Uuid::new_v4();
    let email = format!("test.{}@example.com", unique);

    let register_payload = serde_json::json!({
        "name": format!("Test User {}", unique),
        "email": email,
        "password": super::TEST_PASSWORD
    });

    let resp = client
        .post(format!("{}/users/", super::V1_BASE))
        .header(ContentType::JSON)
        .body(register_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created);

    let body: Value = serde_json::from_str(&resp.into_string().await.expect("register body")).expect("valid json");
    let user_id = body["id"].as_str().expect("user id").to_string();

    let login_payload = serde_json::json!({
        "email": email,
        "password": super::TEST_PASSWORD
    });

    let resp = client
        .post(format!("{}/users/login", super::V1_BASE))
        .header(ContentType::JSON)
        .body(login_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Set default currency to EUR
    let eur_id = get_eur_currency_id(client).await;
    let settings_payload = serde_json::json!({
        "theme": "light",
        "language": "en",
        "default_currency_id": eur_id
    });

    let resp = client
        .put(format!("{}/settings", super::V1_BASE))
        .header(ContentType::JSON)
        .body(settings_payload.to_string())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    (user_id, email)
}
