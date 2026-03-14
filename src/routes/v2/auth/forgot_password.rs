use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::dto::auth::ForgotPasswordRequest;

#[post("/forgot-password", data = "<_payload>")]
pub async fn forgot_password(_payload: Json<ForgotPasswordRequest>) -> Status {
    todo!()
}
