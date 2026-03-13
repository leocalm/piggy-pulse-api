use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::dto::auth::ResetPasswordRequest;

#[post("/reset-password", data = "<_payload>")]
pub async fn reset_password(_payload: Json<ResetPasswordRequest>) -> Status {
    todo!()
}
