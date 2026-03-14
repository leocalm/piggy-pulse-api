use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::dto::auth::EmergencyDisableRequestBody;

#[post("/emergency-disable/request", data = "<_payload>")]
pub async fn emergency_disable_request(_payload: Json<EmergencyDisableRequestBody>) -> Status {
    todo!()
}
