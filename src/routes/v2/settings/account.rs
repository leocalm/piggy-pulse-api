use rocket::delete;
use rocket::http::Status;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::settings::DeleteAccountRequest;

#[delete("/account", data = "<_payload>")]
pub async fn delete_account(_user: CurrentUser, _payload: Json<DeleteAccountRequest>) -> Status {
    todo!()
}
