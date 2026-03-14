use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put};

use crate::auth::CurrentUser;
use crate::dto::period::{CreatePeriodScheduleRequest, PeriodScheduleResponse, UpdatePeriodScheduleRequest};

#[get("/schedule")]
pub async fn get_schedule(_user: CurrentUser) -> Json<PeriodScheduleResponse> {
    todo!()
}

#[post("/schedule", data = "<_payload>")]
pub async fn create_schedule(_user: CurrentUser, _payload: Json<CreatePeriodScheduleRequest>) -> (Status, Json<PeriodScheduleResponse>) {
    todo!()
}

#[put("/schedule", data = "<_payload>")]
pub async fn update_schedule(_user: CurrentUser, _payload: Json<UpdatePeriodScheduleRequest>) -> Json<PeriodScheduleResponse> {
    todo!()
}

#[delete("/schedule")]
pub async fn delete_schedule(_user: CurrentUser) -> Status {
    todo!()
}
