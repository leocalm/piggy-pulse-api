use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::dashboard::CurrentPeriodResponse;

#[get("/current-period?<_period_id>")]
pub async fn get_current_period(_user: CurrentUser, _period_id: Option<String>) -> Json<CurrentPeriodResponse> {
    todo!()
}
