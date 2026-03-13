use rocket::put;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::period::{PeriodResponse, UpdatePeriodRequest};

#[put("/<_id>", data = "<_payload>")]
pub async fn update_period(_user: CurrentUser, _id: &str, _payload: Json<UpdatePeriodRequest>) -> Json<PeriodResponse> {
    todo!()
}
