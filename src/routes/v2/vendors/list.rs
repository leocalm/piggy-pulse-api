use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::vendors::VendorListResponse;

#[get("/?<_cursor>&<_limit>")]
pub async fn list_vendors(_user: CurrentUser, _cursor: Option<String>, _limit: Option<u32>) -> Json<VendorListResponse> {
    todo!()
}
