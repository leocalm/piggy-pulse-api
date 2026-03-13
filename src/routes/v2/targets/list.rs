use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::CategoryTargetsResponse;

#[get("/?<_period_id>")]
pub async fn list_targets(_user: CurrentUser, _period_id: Option<String>) -> Json<CategoryTargetsResponse> {
    todo!()
}
