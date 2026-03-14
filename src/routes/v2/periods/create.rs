use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::period::{CreatePeriodRequest, PeriodResponse};

#[post("/", data = "<_payload>")]
pub async fn create_period(_user: CurrentUser, _payload: Json<CreatePeriodRequest>) -> (Status, Json<PeriodResponse>) {
    todo!()
}
