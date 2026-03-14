use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::vendors::{CreateVendorRequest, VendorResponse};

#[post("/", data = "<_payload>")]
pub async fn create_vendor(_user: CurrentUser, _payload: Json<CreateVendorRequest>) -> (Status, Json<VendorResponse>) {
    todo!()
}
