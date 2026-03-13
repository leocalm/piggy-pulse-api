use rocket::put;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::{CategoryResponse, UpdateCategoryRequest};

#[put("/<_id>", data = "<_payload>")]
pub async fn update_category(_user: CurrentUser, _id: &str, _payload: Json<UpdateCategoryRequest>) -> Json<CategoryResponse> {
    todo!()
}
