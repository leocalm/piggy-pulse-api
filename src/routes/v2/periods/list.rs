use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::period::PeriodListResponse;

#[get("/?<_cursor>&<_limit>")]
pub async fn list_periods(_user: CurrentUser, _cursor: Option<String>, _limit: Option<u32>) -> Json<PeriodListResponse> {
    todo!()
}
