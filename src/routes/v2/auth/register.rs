use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::dto::auth::AuthenticatedResponse;
use crate::dto::auth::RegisterRequest;

#[post("/register", data = "<_payload>")]
pub async fn register(_payload: Json<RegisterRequest>) -> (Status, Json<AuthenticatedResponse>) {
    todo!()
}
