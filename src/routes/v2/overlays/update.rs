use rocket::put;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::overlay::{OverlayResponse, UpdateOverlayRequest};

#[put("/<_id>", data = "<_payload>")]
pub async fn update_overlay(_user: CurrentUser, _id: &str, _payload: Json<UpdateOverlayRequest>) -> Json<OverlayResponse> {
    todo!()
}
