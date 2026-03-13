use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::vendors::VendorOptionListResponse;

#[get("/options")]
pub async fn list_vendor_options(_user: CurrentUser) -> Json<VendorOptionListResponse> {
    todo!()
}
