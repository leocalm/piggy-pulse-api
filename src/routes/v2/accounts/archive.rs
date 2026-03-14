use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::AccountResponse;

#[post("/<_id>/archive")]
pub async fn archive_account(_user: CurrentUser, _id: &str) -> Json<AccountResponse> {
    todo!()
}
