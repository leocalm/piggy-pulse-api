use rocket::put;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::{AccountResponse, UpdateAccountRequest};

#[put("/<_id>", data = "<_payload>")]
pub async fn update_account(_user: CurrentUser, _id: &str, _payload: Json<UpdateAccountRequest>) -> Json<AccountResponse> {
    todo!()
}
