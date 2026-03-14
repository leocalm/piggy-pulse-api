use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::categories::CategoryOptionListResponse;

#[get("/options")]
pub async fn list_category_options(_user: CurrentUser) -> Json<CategoryOptionListResponse> {
    todo!()
}
