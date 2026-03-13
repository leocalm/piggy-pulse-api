use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::settings::ResetStructureRequest;

#[post("/reset-structure", data = "<_payload>")]
pub async fn reset_structure(_user: CurrentUser, _payload: Json<ResetStructureRequest>) -> Status {
    todo!()
}
