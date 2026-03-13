use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::CategoryOverviewResponse;

#[get("/overview?<_period_id>")]
pub async fn category_overview(_user: CurrentUser, _period_id: Option<String>) -> Json<CategoryOverviewResponse> {
    todo!()
}
