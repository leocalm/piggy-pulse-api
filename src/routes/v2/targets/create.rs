use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::{CreateTargetRequest, TargetItem};

#[post("/", data = "<_payload>")]
pub async fn create_target(_user: CurrentUser, _payload: Json<CreateTargetRequest>) -> (Status, Json<TargetItem>) {
    todo!()
}
