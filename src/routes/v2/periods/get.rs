use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::period::PeriodResponse;

#[get("/<_id>")]
pub async fn get_period(_user: CurrentUser, _id: &str) -> Json<PeriodResponse> {
    todo!()
}
