use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::overlay::OverlayResponse;

#[get("/<_id>")]
pub async fn get_overlay(_user: CurrentUser, _id: &str) -> Json<OverlayResponse> {
    todo!()
}
