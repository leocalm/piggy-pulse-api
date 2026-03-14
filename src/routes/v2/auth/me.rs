use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::auth::UserResponse;

#[get("/me")]
pub async fn me(_user: CurrentUser) -> Json<UserResponse> {
    todo!()
}
