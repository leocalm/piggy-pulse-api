use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::auth::RefreshResponse;

#[post("/refresh")]
pub async fn refresh(_user: CurrentUser) -> Json<RefreshResponse> {
    todo!()
}
