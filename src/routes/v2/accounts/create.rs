use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::{AccountResponse, CreateAccountRequest};

#[post("/", data = "<_payload>")]
pub async fn create_account(_user: CurrentUser, _payload: Json<CreateAccountRequest>) -> (Status, Json<AccountResponse>) {
    todo!()
}
