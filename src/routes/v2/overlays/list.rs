use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::overlay::OverlayListResponse;

#[get("/?<_cursor>&<_limit>")]
pub async fn list_overlays(_user: CurrentUser, _cursor: Option<String>, _limit: Option<u32>) -> Json<OverlayListResponse> {
    todo!()
}
