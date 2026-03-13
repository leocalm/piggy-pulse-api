use rocket::post;
use rocket::serde::json::Json;

use crate::dto::auth::LoginRequest;
use crate::dto::auth::LoginResponse;

#[post("/login", data = "<_payload>")]
pub async fn login(_payload: Json<LoginRequest>) -> Json<LoginResponse> {
    todo!()
}
