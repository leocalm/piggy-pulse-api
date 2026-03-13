use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::CategoryResponse;

#[post("/<_id>/unarchive")]
pub async fn unarchive_category(_user: CurrentUser, _id: &str) -> Json<CategoryResponse> {
    todo!()
}
