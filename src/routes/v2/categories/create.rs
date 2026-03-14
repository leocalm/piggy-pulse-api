use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::{CategoryResponse, CreateCategoryRequest};

#[post("/", data = "<_payload>")]
pub async fn create_category(_user: CurrentUser, _payload: Json<CreateCategoryRequest>) -> (Status, Json<CategoryResponse>) {
    todo!()
}
