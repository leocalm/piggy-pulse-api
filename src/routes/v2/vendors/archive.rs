use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::vendors::VendorResponse;

#[post("/<_id>/archive")]
pub async fn archive_vendor(_user: CurrentUser, _id: &str) -> Json<VendorResponse> {
    todo!()
}
