use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::TargetItem;

#[post("/<_id>/exclude")]
pub async fn exclude_target(_user: CurrentUser, _id: &str) -> Json<TargetItem> {
    todo!()
}
