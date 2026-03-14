use rocket::put;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::vendors::{UpdateVendorRequest, VendorResponse};

#[put("/<_id>", data = "<_payload>")]
pub async fn update_vendor(_user: CurrentUser, _id: &str, _payload: Json<UpdateVendorRequest>) -> Json<VendorResponse> {
    todo!()
}
