use rocket::put;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::{TargetItem, UpdateTargetRequest};

#[put("/<_id>", data = "<_payload>")]
pub async fn update_target(_user: CurrentUser, _id: &str, _payload: Json<UpdateTargetRequest>) -> Json<TargetItem> {
    todo!()
}
