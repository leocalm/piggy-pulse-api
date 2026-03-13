use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::overlay::{CreateOverlayRequest, OverlayResponse};

#[post("/", data = "<_payload>")]
pub async fn create_overlay(_user: CurrentUser, _payload: Json<CreateOverlayRequest>) -> (Status, Json<OverlayResponse>) {
    todo!()
}
