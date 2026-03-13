use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::dto::auth::EmergencyDisableConfirmRequest;

#[post("/emergency-disable/confirm", data = "<_payload>")]
pub async fn emergency_disable_confirm(_payload: Json<EmergencyDisableConfirmRequest>) -> Status {
    todo!()
}
