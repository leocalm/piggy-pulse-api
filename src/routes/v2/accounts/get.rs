use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::AccountResponse;

#[get("/<_id>")]
pub async fn get_account(_user: CurrentUser, _id: &str) -> Json<AccountResponse> {
    todo!()
}
